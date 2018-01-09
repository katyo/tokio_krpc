use std::ops::BitXor;

use rand::{Rng, OsRng};

use crypto_hashes::digest::Digest;
use crypto_hashes::sha1::Sha1;

use super::NodeId;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Sha1Id(
    #[serde(with = "serde_hash")]
    [u8; 20]
);

impl Sha1Id {
    pub fn new() -> Self {
        let mut hasher = Sha1::default();
        let mut generator = OsRng::new().unwrap();
        let mut bytes = [0u8; 20];
        generator.fill_bytes(&mut bytes);
        hasher.input(&bytes);
        bytes.clone_from_slice(&hasher.result());
        Sha1Id(bytes)
    }
}

impl AsRef<[u8; 20]> for Sha1Id {
    fn as_ref(&self) -> &[u8; 20] {
        &self.0
    }
}

impl<'a> From<&'a str> for Sha1Id {
    fn from(v: &'a str) -> Self {
        let mut node_id = [0u8; 20];
        node_id.clone_from_slice(v.as_bytes());
        Sha1Id(node_id)
    }
}

impl From<[u8; 20]> for Sha1Id {
    fn from(node_id: [u8; 20]) -> Self {
        Sha1Id(node_id)
    }
}

impl Default for Sha1Id {
    fn default() -> Self {
        Sha1Id([0u8; 20])
    }
}

impl BitXor<Sha1Id> for Sha1Id {
    type Output = Sha1Id;

    fn bitxor(self, other: Self) -> Self {
        let mut out = [0u8; 20];
        for i in 0 .. 20 {
            out[i] = self.0[i] ^ other.0[i];
        }
        Sha1Id(out)
    }
}

impl NodeId for Sha1Id {
    fn equal_bits(&self, other: &Self) -> usize {
        let a = &self.0;
        let b = &other.0;
        if let Some(i) = a.iter().zip(b.iter()).position(|(a, b)| a != b) {
            i * 8 + (a[i] ^ b[i]).leading_zeros() as usize
        } else {
            20 * 8
        }
    }
}

pub mod serde_hash {
    use serde_bytes;
    use serde::ser::Serializer;
    use serde::de::{Deserializer, Error};
    
    pub fn to_bytes(buf: &mut Vec<u8>, hash: &[u8; 20]) {
        buf.extend(hash);
    }

    pub fn from_bytes(buf: &[u8]) -> Result<[u8; 20], ()> {
        let len = buf.len();
        if len == 20 {
            let mut hash = [0u8; 20];
            hash.clone_from_slice(&buf);
            Ok(hash)
        } else {
            Err(())
        }
    }
    
    pub fn serialize<S>(hash: &[u8; 20], serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        serializer.serialize_bytes(hash)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 20], D::Error>
        where D: Deserializer<'de>
    {
        let buf: Vec<u8> = serde_bytes::deserialize(deserializer)?;
        from_bytes(&buf).map_err(|_| Error::custom("Malformed compact node info"))
    }
}

#[cfg(test)]
mod tests {
    use test::{black_box, Bencher};
    use super::{Sha1Id, NodeId};

    #[test]
    pub fn test_hash_xor() {
        assert_eq!(Sha1Id::from([0x00u8; 20]),
                   Sha1Id::from([0x00u8; 20]) ^
                   Sha1Id::from([0x00u8; 20]));

        assert_eq!(Sha1Id::from([0xFFu8; 20]),
                   Sha1Id::from([0xFFu8; 20]) ^
                   Sha1Id::from([0x00u8; 20]));

        assert_eq!(Sha1Id::from([0xFFu8; 20]),
                   Sha1Id::from([0x00u8; 20]) ^
                   Sha1Id::from([0xFFu8; 20]));

        assert_eq!(Sha1Id::from([0x00u8; 20]),
                   Sha1Id::from([0xFFu8; 20]) ^
                   Sha1Id::from([0xFFu8; 20]));

        assert_eq!(Sha1Id::from([0x55u8; 20]),
                   Sha1Id::from([0x00u8; 20]) ^
                   Sha1Id::from([0x55u8; 20]));

        assert_eq!(Sha1Id::from([0xAAu8; 20]),
                   Sha1Id::from([0xFFu8; 20]) ^
                   Sha1Id::from([0x55u8; 20]));
        
        assert_eq!(Sha1Id::from([0xFFu8; 20]),
                   Sha1Id::from([0xAAu8; 20]) ^
                   Sha1Id::from([0x55u8; 20]));
    }

    #[bench]
    pub fn bench_hash_xor(b: &mut Bencher) {
        let x = Sha1Id::from([0xAAu8; 20]);
        let y = Sha1Id::from([0x55u8; 20]);

        b.iter(|| {
            (0..black_box(1000)).fold(x, |a, _| { a ^ y; a })
        });
    }

    #[test]
    pub fn test_hash_beq() {
        assert_eq!(0,
                   Sha1Id::from([0xFFu8; 20])
                   .equal_bits(&Sha1Id::from([0x00u8; 20])));

        assert_eq!(0,
                   Sha1Id::from([0x00u8; 20])
                   .equal_bits(&Sha1Id::from([0xFFu8; 20])));
        
        assert_eq!(0,
                   Sha1Id::from([0xAAu8; 20])
                   .equal_bits(&Sha1Id::from([0x55u8; 20])));

        assert_eq!(1,
                   Sha1Id::from([0x00u8; 20])
                   .equal_bits(&Sha1Id::from([0x55u8; 20])));

        assert_eq!(1,
                   Sha1Id::from([0xFFu8; 20])
                   .equal_bits(&Sha1Id::from([0xAAu8; 20])));
        
        assert_eq!(160,
                   Sha1Id::from([0x00u8; 20])
                   .equal_bits(&Sha1Id::from([0x00u8; 20])));

        assert_eq!(160,
                   Sha1Id::from([0xFFu8; 20])
                   .equal_bits(&Sha1Id::from([0xFFu8; 20])));

        assert_eq!(160,
                   Sha1Id::from([0x55u8; 20])
                   .equal_bits(&Sha1Id::from([0x55u8; 20])));

        assert_eq!(160,
                   Sha1Id::from([0xAAu8; 20])
                   .equal_bits(&Sha1Id::from([0xAAu8; 20])));

        assert_eq!(21,
                   Sha1Id::from([0x01, 0x23, 0x45, 0x67, 0x78, 0x90, 0xab, 0xcd, 0xef, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
                   .equal_bits(&Sha1Id::from([0x01, 0x23, 0x41, 0x67, 0x78, 0x90, 0xab, 0xef, 0xcd, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])));

        assert_eq!(75,
                   Sha1Id::from([0x01, 0x23, 0x45, 0x67, 0x78, 0x90, 0xab, 0xcd, 0xef, 0xa5, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
                   .equal_bits(&Sha1Id::from([0x01, 0x23, 0x45, 0x67, 0x78, 0x90, 0xab, 0xcd, 0xef, 0xb5, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])));
    }

    #[bench]
    pub fn bench_hash_beq(b: &mut Bencher) {
        let x = Sha1Id::from([0xAAu8; 20]);
        let y = Sha1Id::from([0x55u8; 20]);
        let mut t = 0;

        b.iter(|| {
            (0..black_box(1000)).fold(x, |a, _| { t += a.equal_bits(&y); a })
        });
    }
}
