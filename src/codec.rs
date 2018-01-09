use std::net::SocketAddr;
use std::io::{Error, ErrorKind, Result};
use std::fmt::Debug;
use std::marker::PhantomData;

use serde::ser::Serialize;
use serde::de::DeserializeOwned;

use hexdump::hexdump_iter;

use serde_bencode::ser::to_bytes;
use serde_bencode::de::from_bytes;

use tokio_core::net::UdpCodec;

use super::{KMessage, KAddress, KTransId, KError, KQueryArg};

pub struct KCodec<Query, Arg, Res> {
    phantom: PhantomData<(Query, Arg, Res)>,
}

impl<Query, Arg, Res> KCodec<Query, Arg, Res>
    where Query: Serialize + DeserializeOwned,
          Arg: Serialize + DeserializeOwned,
          Res: Serialize + DeserializeOwned,
{
    pub fn new() -> Self {
        KCodec {
            phantom: PhantomData,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KId(pub SocketAddr, pub Option<KTransId>);

#[derive(Debug, Clone)]
pub enum KData<Arg, Res> {
    Query(Arg),
    Response(Res),
    Error(KError),
}

#[derive(Debug, Clone)]
pub struct KItem<Arg, Res>(pub KId, pub KData<Arg, Res>);

impl<Arg, Res> Eq for KItem<Arg, Res> {}

impl<Arg, Res> PartialEq for KItem<Arg, Res> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<Query, Arg, Res> UdpCodec for KCodec<Query, Arg, Res>
    where Query: Serialize + DeserializeOwned + Debug + Eq,
          Arg: Serialize + DeserializeOwned + Debug + KQueryArg<Query = Query>,
          Res: Serialize + DeserializeOwned + Debug,
{
    type In = KItem<Arg, Res>;
    type Out = KItem<Arg, Res>;

    fn decode(&mut self, addr: &SocketAddr, buf: &[u8]) -> Result<Self::In> {
        trace!("recv from: {}, packet:", addr);
        for line in hexdump_iter(buf) {
            trace!("    {}", line);
        }
        let msg: KMessage<Query, Arg, Res> = from_bytes(buf)
            .map_err(|err| Error::new(ErrorKind::InvalidData,
                                      format!("Decode error: {}", err)))?;
        debug!("recv from: {}, message: {:?}", addr, msg);
        match msg {
            KMessage::Query {tid, query, arg} => {
                if arg.query() == query {
                    Ok(KItem(KId(*addr, tid), KData::Query(arg)))
                } else {
                    Err(Error::new(ErrorKind::InvalidData,
                                   "Malformed message"))
                }
            },
            KMessage::Response {tid, res, ..} =>
                Ok(KItem(KId(*addr, tid), KData::Response(res))),
            KMessage::Error {tid, error, ..} =>
                Ok(KItem(KId(*addr, tid), KData::Error(error))),
        }
    }

    fn encode(&mut self, KItem(KId(addr, tid), msg): Self::Out, into: &mut Vec<u8>) -> SocketAddr {
        debug!("send to: {}, message: {:?}", addr, msg);
        let msg = match msg {
            KData::Query(arg) => KMessage::Query {tid, query: arg.query(), arg},
            KData::Response(res) => KMessage::Response {ip: Some(KAddress(addr)), tid, res},
            KData::Error(error) => KMessage::Error {ip: Some(KAddress(addr)), tid, error},
        };
        let buf = to_bytes(&msg).unwrap();
        trace!("send to: {}, packet:", addr);
        for line in hexdump_iter(&buf) {
            trace!("    {}", line);
        }
        into.extend(buf);
        addr
    }
}
