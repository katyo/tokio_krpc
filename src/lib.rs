#![feature(box_syntax)]
#![feature(conservative_impl_trait)]
#![feature(test)]
extern crate test;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_bytes;
extern crate serde_bencode;

#[macro_use]
extern crate log;

extern crate hexdump;

extern crate futures;
extern crate tokio_core;
extern crate tokio_service;

extern crate rand;
extern crate crypto_hashes;

#[macro_use]
pub mod serde_extra;

pub mod rpc;
pub mod codec;
pub mod trans;
pub mod service;
pub mod dht;

pub use self::rpc::{KAddress, KTransId, KMessage, KError, KErrorKind, KQueryArg};
pub use self::codec::{KCodec, KItem, KId, KData};
pub use self::trans::{KTrans};
pub use self::service::{KTransError, KOptions, KService};
