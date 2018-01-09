use std::net::SocketAddr;
use serde_bytes;
use serde_extra::socket_addr;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct KAddress (
    #[serde(with = "socket_addr")]
    pub SocketAddr,
);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct KTransId (
    #[serde(with = "serde_bytes")]
    pub Vec<u8>,
);

impl<'a> From<&'a str> for KTransId {
    fn from(s: &'a str) -> Self {
        KTransId(s.into())
    }
}

impl<'a> From<&'a [u8]> for KTransId {
    fn from(b: &'a [u8]) -> Self {
        KTransId(b.into())
    }
}

impl AsRef<[u8]> for KTransId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "y")]
pub enum KMessage<Query, Arg, Res> {
    #[serde(rename = "q")]
    Query {
        #[serde(rename = "t")]
        tid: Option<KTransId>,
        #[serde(rename = "q")]
        query: Query,
        #[serde(rename = "a")]
        arg: Arg,
    },
    #[serde(rename = "r")]
    Response {
        ip: Option<KAddress>,
        #[serde(rename = "t")]
        tid: Option<KTransId>,
        #[serde(rename = "r")]
        res: Res,
    },
    #[serde(rename = "e")]
    Error {
        ip: Option<KAddress>,
        #[serde(rename = "t")]
        tid: Option<KTransId>,
        #[serde(rename = "e")]
        error: KError,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct KError(
    pub KErrorKind,
    pub String,
);

serde_numeric_enum!(KErrorKind {
    Generic = 201,
    Server = 202,
    Protocol = 203,
    Method = 204,
});

pub trait KQueryArg {
    type Query;
    
    fn query(&self) -> Self::Query;
}
