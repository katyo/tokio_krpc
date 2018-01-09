use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Duration;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;

use serde::ser::Serialize;
use serde::de::DeserializeOwned;

use futures::{Future, Sink, Stream};
use futures::future::{Either, Loop, loop_fn, ok, err};
use futures::unsync::{oneshot, mpsc};

use tokio_core::reactor::{Handle, Timeout};
use tokio_core::net::UdpSocket;
use tokio_service::Service;

use super::{KError, KQueryArg, KCodec, KItem, KData, KTrans, KId};

#[derive(Debug)]
pub enum KTransError {
    KError(KError),
    IOError(Error),
    Timeout,
}

type KTransResponder<Res> = oneshot::Sender<Result<Res, KTransError>>;
type KTransIdenter = oneshot::Sender<KId>;
struct KTransQuery<Arg, Res>(SocketAddr, Arg, KTransResponder<Res>, KTransIdenter);

#[derive(Debug, Clone)]
pub struct KOptions {
    pub timeout: Duration,
}

#[derive(Clone)]
pub struct KService<Query, Arg, Res, Handler> {
    options: KOptions,
    query_tx: mpsc::Sender<Either<KTransQuery<Arg, Res>, KId>>,
    handle: Handle,
    phantom: PhantomData<(Query, Handler)>,
}

impl<'s, Query, Arg, Res, Handler> KService<Query, Arg, Res, Handler>
    where Query: 's + Serialize + DeserializeOwned + Debug + Eq,
          Arg: 's + Serialize + DeserializeOwned + Debug + KQueryArg<Query = Query>,
          Res: 's + Serialize + DeserializeOwned + Debug,
          Handler: 's + Service<Request = Arg, Response = Res, Error = KError>,
{
    pub fn new(handler: Handler, addr: &SocketAddr, handle: &Handle, options: KOptions) -> (Self, impl Future<Item = (), Error = Error> + 's) {
        let trans: KTrans<KTransResponder<Res>> = KTrans::new();
        let codec: KCodec<Query, Arg, Res> = KCodec::new();
        let socket = UdpSocket::bind(addr, handle).unwrap();
        let handle = handle.clone();
        
        info!("Listening on: {}", socket.local_addr().unwrap());
        
        let (net_tx, net_rx) = socket.framed(codec).split();
        let (query_tx, query_rx) = mpsc::channel(1);
        // Compose event stream
        let event_rx = net_rx.map(Either::A)
            .select(query_rx.map(Either::B)
                    .map_err(|_| Error::new(ErrorKind::Other, "Query error")))
            .into_future();
        (KService { options, query_tx, handle, phantom: PhantomData },
         loop_fn((event_rx, net_tx, trans, handler),
                 |(event_rx, net_tx, mut trans, handler)| {
                     event_rx.map_err(|(err, ..)| {
                         error!("recv err: {}", err);
                         err
                     }).and_then(|(item, event_stream)| {
                         if let Some(item) = item {
                             let event_rx = event_stream.into_future();
                             match item {
                                 Either::A(KItem(trans_id, msg)) => {
                                     match msg {
                                         KData::Query(arg) => {
                                             return Either::B(Either::A(handler.call(arg).then(|result| {
                                                 let resp = match result {
                                                     Ok(res) => KData::Response(res),
                                                     Err(err) => KData::Error(err),
                                                 };
                                                 net_tx.send(KItem(trans_id, resp))
                                                     .and_then(|net_tx| {
                                                         ok(Loop::Continue((event_rx, net_tx, trans, handler)))
                                                     })
                                             })));
                                         },
                                         KData::Response(res) => {
                                             if let Some(res_tx) = trans.end(&trans_id) {
                                                 let _ = res_tx.send(Ok(res));
                                             }
                                         },
                                         KData::Error(err) => {
                                             warn!("Received KRPC error: {:?}", err);
                                             if let Some(res_tx) = trans.end(&trans_id) {
                                                 let _ = res_tx.send(Err(KTransError::KError(err)));
                                             }
                                         },
                                     }
                                 },
                                 Either::B(Either::A(KTransQuery(addr, arg, res_tx, tid_tx))) => {
                                     let trans_id = trans.start(addr, res_tx);
                                     let _ = tid_tx.send(trans_id.clone());
                                     return Either::B(Either::B(
                                         net_tx.send(KItem(trans_id, KData::Query(arg)))
                                             .and_then(|net_tx| {
                                                 ok(Loop::Continue((event_rx, net_tx, trans, handler)))
                                             })))
                                 },
                                 Either::B(Either::B(trans_id)) => {
                                     warn!("DHT Response timeout");
                                     if let Some(res_tx) = trans.end(&trans_id) {
                                         let _ = res_tx.send(Err(KTransError::Timeout));
                                     }
                                 },
                             }
                             Either::A(ok(Loop::Continue((event_rx, net_tx, trans, handler))))
                         } else {
                             Either::A(ok(Loop::Break(())))
                         }
                     })
                 }
         ))
    }

    // At the moment some objective difficulties didn't allow implement tokio_service::Service trait directly.
    pub fn call(&self, addr: SocketAddr, arg: Arg) -> impl Future<Item = Res, Error = KTransError> {
        let KOptions {timeout, ..} = self.options;
        let (res_tx, res_rx) = oneshot::channel();
        let (tid_tx, tid_rx) = oneshot::channel();
        let handle = self.handle.clone();
        let cancel_tx = self.query_tx.clone();
        let query_tx = self.query_tx.clone();

        query_tx.send(Either::A(KTransQuery(addr, arg, res_tx, tid_tx)))
            .map_err(|_| KTransError::IOError(Error::new(ErrorKind::Other, "Send error")))
            .and_then(move |_| {
                tid_rx.map_err(|_| KTransError::IOError(Error::new(ErrorKind::Other, "Send error")))
                    .and_then(move |tid| {
                        Timeout::new(timeout, &handle).unwrap()
                            .map_err(|err| KTransError::IOError(err))
                            .map(Either::B)
                            .select(res_rx.map_err(|_| KTransError::IOError(Error::new(ErrorKind::Other, "Recv error")))
                                    .map(Either::A))
                            .map_err(|(err, _)| err)
                            .and_then(|(result, _)| {
                                match result {
                                    Either::A(Ok(res)) => Either::A(ok(res)),
                                    Either::A(Err(error)) => Either::A(err(error)),
                                    Either::B(_) => Either::B(cancel_tx.send(Either::B(tid))
                                                              .then(|_| err(KTransError::Timeout))),
                                }
                            })
                    })
            })
    }
}
