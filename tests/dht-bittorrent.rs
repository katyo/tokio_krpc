#![feature(conservative_impl_trait)]
extern crate futures;
extern crate tokio_core;
extern crate tokio_service;
extern crate tokio_krpc;

#[macro_use]
extern crate log;

use std::net::SocketAddr;
use std::io::Error;
use std::time::Duration;

use futures::{Future};
use futures::future::{ok, err};

use tokio_core::reactor::{Handle, Core};
use tokio_service::Service;

use tokio_krpc::{KError, KErrorKind, KService, KTransError, KOptions};
use tokio_krpc::dht::bittorrent::{BtDhtId, BtDhtQuery, BtDhtArg, BtDhtRes};

#[derive(Clone)]
pub struct BtDhtHandler {
    node_id: BtDhtId,
}

impl BtDhtHandler {
    pub fn new(node_id: BtDhtId) -> Self {
        BtDhtHandler { node_id }
    }
}

impl Service for BtDhtHandler {
    type Request = BtDhtArg;
    type Response = BtDhtRes;
    type Error = KError;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;
    
    fn call(&self, arg: Self::Request) -> Self::Future {
        Box::new(match arg {
            BtDhtArg::Ping {id} => {
                info!("Received ping query from: {:?}", id);
                ok(BtDhtRes::Pong {id: self.node_id})
            },
            _ => {
                err(KError(KErrorKind::Method, "Method unimplemented".into()))
            },
        })
    }
}

#[derive(Clone)]
pub struct BtDhtService {
    node_id: BtDhtId,
    service: KService<BtDhtQuery, BtDhtArg, BtDhtRes, BtDhtHandler>,
}

#[derive(Debug)]
pub enum BtDhtError {
    TransError(KTransError),
    InvalidResponse,
    NotFound,
}

impl<'s> BtDhtService {
    pub fn new(node_id: BtDhtId, addr: &SocketAddr, handle: &Handle) -> (Self, impl Future<Item = (), Error = Error> + 's) {
        let handler = BtDhtHandler::new(node_id);
        let options = KOptions { timeout: Duration::from_secs(2) };
        let (service, thread) = KService::new(handler, addr, handle, options);
        (BtDhtService {node_id, service}, thread)
    }

    pub fn ping_node(&self, addr: SocketAddr) -> impl Future<Item = BtDhtId, Error = BtDhtError> + 's {
        info!("Send ping query to: {:?}", addr);
        self.service.call(addr, BtDhtArg::Ping {id: self.node_id})
            .map_err(BtDhtError::TransError)
            .and_then(move |res| {
                match res {
                    BtDhtRes::Pong {id} => {
                        info!("Received ping response from: {:?} with id: {:?}", addr, id);
                        ok(id)
                    },
                    resp => {
                        warn!("Received invalid response to ping: {:?}", resp);
                        err(BtDhtError::InvalidResponse)
                    },
                }
            }).or_else(|error| {
                info!("Unable to receive ping response due to: {:?}", error);
                err(error)
            })
    }
}

#[test]
fn test_ping_query() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let node1_addr = "127.0.0.1:6881".parse().unwrap();
    let node2_addr = "127.0.0.1:6882".parse().unwrap();
    
    let node1_id = BtDhtId::new();
    let node2_id = BtDhtId::new();

    let (node1_service, node1_server) = BtDhtService::new(node1_id, &node1_addr, &handle);
    let (node2_service, node2_server) = BtDhtService::new(node2_id, &node2_addr, &handle);

    handle.spawn(node1_server.map_err(|_| ()));
    handle.spawn(node2_server.map_err(|_| ()));
    
    core.run(node1_service.ping_node(node2_addr)
             .map(|peer_id| {
                 assert_eq!(peer_id, node2_id);
                 peer_id
             })
             .join(node2_service.ping_node(node1_addr)
                   .map(|peer_id| {
                       assert_eq!(peer_id, node1_id);
                       peer_id
                   }))
             .map_err(|_| ())).unwrap();
}
