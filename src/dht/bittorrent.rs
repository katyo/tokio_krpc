use serde_bytes;
use serde::de::{Deserialize, Deserializer};

use std::net::SocketAddr;
use std::str::from_utf8;

use rpc::KQueryArg;
use serde_extra::{socket_addr, option_bool};

use super::id::Sha1Id;

pub type BtDhtId = Sha1Id;

#[derive(Serialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum BtDhtQuery {
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "find_node")]
    FindNode,
    #[serde(rename = "get_peers")]
    GetPeers,
    #[serde(rename = "announce_peer")]
    AnnouncePeer,
}

impl<'de> Deserialize<'de> for BtDhtQuery {
    fn deserialize<D>(deserializer: D) -> Result<BtDhtQuery, D::Error>
        where D: Deserializer<'de>
    {
        use serde::de::Error;
        let buf: Vec<u8> = serde_bytes::deserialize(deserializer)?;
        let name = from_utf8(&buf).map_err(|_| Error::custom("Invalid method"))?;
        match name {
            "ping" => Ok(BtDhtQuery::Ping),
            "find_node" => Ok(BtDhtQuery::FindNode),
            "get_peers" => Ok(BtDhtQuery::GetPeers),
            "announce_peer" => Ok(BtDhtQuery::AnnouncePeer),
            _ => Err(Error::custom("Unsupported method")),
        }
    }
}

pub type BtDhtToken = Vec<u8>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum BtDhtArg {
    AnnouncePeer {
        id: BtDhtId,
        #[serde(with = "option_bool")]
        implied_port: bool,
        info_hash: BtDhtId,
        port: u16,
        #[serde(with = "serde_bytes")]
        token: BtDhtToken,
    },
    GetPeers {
        id: BtDhtId,
        info_hash: BtDhtId,
    },
    FindNode {
        id: BtDhtId,
        target: BtDhtId,
    },
    Ping {
        id: BtDhtId,
    },
}

impl KQueryArg for BtDhtArg {
    type Query = BtDhtQuery;
    fn query(&self) -> Self::Query {
        match self {
            &BtDhtArg::Ping {..} => BtDhtQuery::Ping,
            &BtDhtArg::FindNode {..} => BtDhtQuery::FindNode,
            &BtDhtArg::GetPeers {..} => BtDhtQuery::GetPeers,
            &BtDhtArg::AnnouncePeer {..} => BtDhtQuery::AnnouncePeer,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum BtDhtRes {
    GetPeersNodes {
        id: BtDhtId,
        #[serde(with = "serde_bytes")]
        token: BtDhtToken,
        #[serde(with = "nodes_info")]
        nodes: BtDhtNodesInfo,
    },
    GetPeersValues {
        id: BtDhtId,
        #[serde(with = "serde_bytes")]
        token: BtDhtToken,
        values: BtDhtPeersInfo,
    },
    FindNode {
        id: BtDhtId,
        #[serde(with = "nodes_info")]
        nodes: BtDhtNodesInfo,
    },
    Pong {
        id: BtDhtId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BtDhtNodeInfo {
    pub id: BtDhtId,
    pub addr: SocketAddr,
}

pub type BtDhtNodesInfo = Vec<BtDhtNodeInfo>;

mod nodes_info {
    use super::{BtDhtId, BtDhtNodeInfo, BtDhtNodesInfo};
    use super::socket_addr;
    use serde_bytes;
    use serde::ser::Serializer;
    use serde::de::{Deserializer, Error};
    use super::super::id::sha1::serde_hash;
    
    pub fn serialize<S>(nodes_info: &BtDhtNodesInfo, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut buf = Vec::new();
        for node_info in nodes_info {
            serde_hash::to_bytes(&mut buf, node_info.id.as_ref());
            socket_addr::to_bytes(&mut buf, &node_info.addr);
        }
        serializer.serialize_bytes(&buf)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BtDhtNodesInfo, D::Error>
        where D: Deserializer<'de>
    {
        let buf: Vec<u8> = serde_bytes::deserialize(deserializer)?;
        let len = buf.len();
        if len % 26 == 0 {
            let mut nodes_info = Vec::new();
            for buf in buf.chunks(26) {
                let mut hash = [0u8; 20];
                hash.clone_from_slice(&buf[..20]);
                let id = BtDhtId::from(hash);
                let addr = socket_addr::from_bytes(&buf[20..]).unwrap();
                nodes_info.push(BtDhtNodeInfo {id, addr});
            }
            Ok(nodes_info)
        } else {
            Err(Error::custom("Malformed compact node info"))
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct BtDhtPeerInfo {
    #[serde(with = "socket_addr")]
    pub addr: SocketAddr,
}

pub type BtDhtPeersInfo = Vec<BtDhtPeerInfo>;

#[cfg(test)]
mod tests {
    use serde_bencode::ser::{to_bytes};
    use serde_bencode::de::{from_bytes};
    use hexdump::hexdump;
    use rpc::{KAddress, KMessage, KError, KErrorKind};
    use super::{BtDhtQuery, BtDhtArg, BtDhtRes};

    type BtDhtMessage = KMessage<BtDhtQuery, BtDhtArg, BtDhtRes>;

    #[test]
    pub fn test_serde_ping_query() {
        let ping_query: BtDhtMessage = KMessage::Query {
            tid: Some("aa".into()),
            query: BtDhtQuery::Ping,
            arg: BtDhtArg::Ping {
                id: "0123456789abcdefghij".into(),
            },
        };

        let ping_query_enc = to_bytes(&ping_query).unwrap();

        println!("ping_query enc:");
        hexdump(&ping_query_enc);

        assert_eq!(r#"d1:ad2:id20:0123456789abcdefghije1:q4:ping1:t2:aa1:y1:qe"#.as_bytes().to_vec(), ping_query_enc);

        let ping_query_dec: BtDhtMessage = from_bytes(&ping_query_enc).unwrap();

        println!("ping_query dec: {:?}", ping_query_dec);
        assert_eq!(ping_query_dec, ping_query);

        //assert!(false);
    }
    
    #[test]
    pub fn test_serde_ping_response() {
        let ping_response: BtDhtMessage = KMessage::Response {
            ip: Some(KAddress("1.2.3.4:56789".parse().unwrap())),
            //ip: None,
            tid: Some("aa".into()),
            res: BtDhtRes::Pong {
                id: "0123456789abcdefghij".into(),
            },
        };

        let ping_response_enc = to_bytes(&ping_response).unwrap();

        println!("ping_response enc:");
        hexdump(&ping_response_enc);

        assert_eq!(vec![100, 50, 58, 105, 112, 54, 58, 1, 2, 3, 4, 221, 213, 49, 58, 114, 100, 50, 58, 105, 100, 50, 48, 58, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 101, 49, 58, 116, 50, 58, 97, 97, 49, 58, 121, 49, 58, 114, 101], ping_response_enc);

        let ping_response_dec: BtDhtMessage = from_bytes(&ping_response_enc).unwrap();

        println!("ping_response dec: {:?}", ping_response_dec);
        assert_eq!(ping_response_dec, ping_response);
    }

    #[test]
    pub fn test_serde_method_error() {
        let method_error: BtDhtMessage = KMessage::Error {
            ip: None,
            tid: Some("55".into()),
            error: KError(KErrorKind::Method, "Unsupported method".into()),
        };

        let method_error_enc = to_bytes(&method_error).unwrap();

        println!("method_error enc:");
        hexdump(&method_error_enc);

        assert_eq!(r#"d1:eli204e18:Unsupported methode1:t2:551:y1:ee"#.as_bytes().to_vec(), method_error_enc);

        let method_error_dec: BtDhtMessage = from_bytes(&method_error_enc).unwrap();

        println!("method_error dec: {:?}", method_error_dec);
        assert_eq!(method_error_dec, method_error);
    }
}
