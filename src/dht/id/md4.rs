use std::ops::BitXor;

use rand::{Rng, OsRng};

use crypto_hashes::digest::Digest;
use crypto_hashes::md4::Md4;

use super::NodeId;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Md4Id(
    #[serde(with = "serde_hash")]
    [u8; 16]
);

impl Md4Id {
    pub fn new() -> Self {
        let mut hasher = Md4::default();
        let mut generator = OsRng::new().unwrap();
        let mut bytes = [0u8; 16];
        generator.fill_bytes(&mut bytes);
        hasher.input(&bytes);
        bytes.clone_from_slice(&hasher.result());
        Md4Id(bytes)
    }
}

impl AsRef<[u8; 16]> for Md4Id {
    fn as_ref(&self) -> &[u8; 16] {
        &self.0
    }
}

impl<'a> From<&'a str> for Md4Id {
    fn from(v: &'a str) -> Self {
        let mut node_id = [0u8; 16];
        node_id.clone_from_slice(v.as_bytes());
        Md4Id(node_id)
    }
}

impl From<[u8; 16]> for Md4Id {
    fn from(node_id: [u8; 16]) -> Self {
        Md4Id(node_id)
    }
}

impl Default for Md4Id {
    fn default() -> Self {
        Md4Id([0u8; 16])
    }
}

impl BitXor<Md4Id> for Md4Id {
    type Output = Md4Id;

    fn bitxor(self, other: Self) -> Self {
        let mut out = [0u8; 16];
        for i in 0 .. 16 {
            out[i] = self.0[i] ^ other.0[i];
        }
        Md4Id(out)
    }
}

impl NodeId for Md4Id {
    fn equal_bits(&self, other: &Self) -> usize {
        let a = &self.0;
        let b = &other.0;
        if let Some(i) = a.iter().zip(b.iter()).position(|(a, b)| a != b) {
            i * 8 + (a[i] ^ b[i]).leading_zeros() as usize
        } else {
            16 * 8
        }
    }
}

pub mod serde_hash {
    use serde_bytes;
    use serde::ser::Serializer;
    use serde::de::{Deserializer, Error};
    
    pub fn to_bytes(buf: &mut Vec<u8>, hash: &[u8; 16]) {
        buf.extend(hash);
    }

    pub fn from_bytes(buf: &[u8]) -> Result<[u8; 16], ()> {
        let len = buf.len();
        if len == 16 {
            let mut hash = [0u8; 16];
            hash.clone_from_slice(&buf);
            Ok(hash)
        } else {
            Err(())
        }
    }
    
    pub fn serialize<S>(hash: &[u8; 16], serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        serializer.serialize_bytes(hash)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 16], D::Error>
        where D: Deserializer<'de>
    {
        let buf: Vec<u8> = serde_bytes::deserialize(deserializer)?;
        from_bytes(&buf).map_err(|_| Error::custom("Malformed compact node info"))
    }
}

#[cfg(test)]
mod tests {
    use test::{black_box, Bencher};
    use super::{Md4Id, NodeId};

    #[test]
    pub fn test_hash_xor() {
        assert_eq!(Md4Id::from([0x00u8; 16]),
                   Md4Id::from([0x00u8; 16]) ^
                   Md4Id::from([0x00u8; 16]));

        assert_eq!(Md4Id::from([0xFFu8; 16]),
                   Md4Id::from([0xFFu8; 16]) ^
                   Md4Id::from([0x00u8; 16]));

        assert_eq!(Md4Id::from([0xFFu8; 16]),
                   Md4Id::from([0x00u8; 16]) ^
                   Md4Id::from([0xFFu8; 16]));

        assert_eq!(Md4Id::from([0x00u8; 16]),
                   Md4Id::from([0xFFu8; 16]) ^
                   Md4Id::from([0xFFu8; 16]));

        assert_eq!(Md4Id::from([0x55u8; 16]),
                   Md4Id::from([0x00u8; 16]) ^
                   Md4Id::from([0x55u8; 16]));

        assert_eq!(Md4Id::from([0xAAu8; 16]),
                   Md4Id::from([0xFFu8; 16]) ^
                   Md4Id::from([0x55u8; 16]));
        
        assert_eq!(Md4Id::from([0xFFu8; 16]),
                   Md4Id::from([0xAAu8; 16]) ^
                   Md4Id::from([0x55u8; 16]));
    }

    #[bench]
    pub fn bench_hash_xor(b: &mut Bencher) {
        let x = Md4Id::from([0xAAu8; 16]);
        let y = Md4Id::from([0x55u8; 16]);

        b.iter(|| {
            (0..black_box(1000)).fold(x, |a, _| { a ^ y; a })
        });
    }

    #[test]
    pub fn test_hash_beq() {
        assert_eq!(0,
                   Md4Id::from([0xFFu8; 16])
                   .equal_bits(&Md4Id::from([0x00u8; 16])));

        assert_eq!(0,
                   Md4Id::from([0x00u8; 16])
                   .equal_bits(&Md4Id::from([0xFFu8; 16])));
        
        assert_eq!(0,
                   Md4Id::from([0xAAu8; 16])
                   .equal_bits(&Md4Id::from([0x55u8; 16])));

        assert_eq!(1,
                   Md4Id::from([0x00u8; 16])
                   .equal_bits(&Md4Id::from([0x55u8; 16])));

        assert_eq!(1,
                   Md4Id::from([0xFFu8; 16])
                   .equal_bits(&Md4Id::from([0xAAu8; 16])));
        
        assert_eq!(128,
                   Md4Id::from([0x00u8; 16])
                   .equal_bits(&Md4Id::from([0x00u8; 16])));

        assert_eq!(128,
                   Md4Id::from([0xFFu8; 16])
                   .equal_bits(&Md4Id::from([0xFFu8; 16])));

        assert_eq!(128,
                   Md4Id::from([0x55u8; 16])
                   .equal_bits(&Md4Id::from([0x55u8; 16])));

        assert_eq!(128,
                   Md4Id::from([0xAAu8; 16])
                   .equal_bits(&Md4Id::from([0xAAu8; 16])));

        assert_eq!(21,
                   Md4Id::from([0x01, 0x23, 0x45, 0x67, 0x78, 0x90, 0xab, 0xcd, 0xef, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
                   .equal_bits(&Md4Id::from([0x01, 0x23, 0x41, 0x67, 0x78, 0x90, 0xab, 0xef, 0xcd, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])));

        assert_eq!(75,
                   Md4Id::from([0x01, 0x23, 0x45, 0x67, 0x78, 0x90, 0xab, 0xcd, 0xef, 0xa5, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
                   .equal_bits(&Md4Id::from([0x01, 0x23, 0x45, 0x67, 0x78, 0x90, 0xab, 0xcd, 0xef, 0xb5, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])));
    }

    #[bench]
    pub fn bench_hash_beq(b: &mut Bencher) {
        let x = Md4Id::from([0xAAu8; 16]);
        let y = Md4Id::from([0x55u8; 16]);
        let mut t = 0;

        b.iter(|| {
            (0..black_box(1000)).fold(x, |a, _| { t += a.equal_bits(&y); a })
        });
    }
}
