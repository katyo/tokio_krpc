use serde::ser::Serializer;
use serde::de::{Deserialize, Deserializer};

pub fn serialize<S>(option: &bool, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer
{
    if *option {
        serializer.serialize_some(&1u8)
    } else {
        serializer.serialize_none()
    }
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where D: Deserializer<'de>
{
    let opt: Option<u8> = Option::deserialize(deserializer)?;
    match opt {
        Some(ref val) if *val == 1u8 => Ok(true),
        _ => Ok(false),
    }
}
