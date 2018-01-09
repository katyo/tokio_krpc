use std::net::{SocketAddr, IpAddr, Ipv4Addr};

use serde_bytes;
use serde::ser::Serializer;
use serde::de::{Deserializer, Error};

pub fn to_bytes(buf: &mut Vec<u8>, addr: &SocketAddr) {
    match addr {
        &SocketAddr::V4(v4) => {
            buf.extend(&v4.ip().octets());
            let port = v4.port();
            buf.push((port >> 8) as u8);
            buf.push((port & 0xff) as u8);
        },
        &SocketAddr::V6(_v6) => {
            // not implemented
        },
    };
}

pub fn from_bytes(buf: &[u8]) -> Result<SocketAddr, ()> {
    match buf.len() {
        6 => {
            let addr = IpAddr::V4(Ipv4Addr::new(buf[0], buf[1], buf[2], buf[3]));
            let port = ((buf[4] as u16) << 8) | (buf[5] as u16);
            Ok(SocketAddr::new(addr, port))
        },
        _ => {
            Err(())
        }
    }
}

pub fn serialize<S>(addr: &SocketAddr, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer
{
    let mut buf = Vec::new();
    to_bytes(&mut buf, addr);
    serializer.serialize_bytes(&buf)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<SocketAddr, D::Error>
    where D: Deserializer<'de>
{
    let buf: Vec<u8> = serde_bytes::deserialize(deserializer)?;
    from_bytes(&buf).map_err(|_| Error::custom("invalid socket addr"))
}
