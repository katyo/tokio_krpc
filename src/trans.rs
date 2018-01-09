use std::net::SocketAddr;
use std::collections::HashMap;

use super::{KTransId, KId};

type TransId = u16;
type TransKey = (SocketAddr, TransId);
type TransPool<Data> = HashMap<TransKey, Data>;

pub struct KTrans<Data> {
    last_tid: TransId,
    pool: TransPool<Data>,
}

impl<Data> KTrans<Data> {
    pub fn new() -> Self {
        KTrans {last_tid: 0, pool: HashMap::new()}
    }

    pub fn active(&self) -> usize {
        self.pool.len()
    }

    pub fn start(&mut self, addr: SocketAddr, data: Data) -> KId {
        self.last_tid += 1;
        let tid = self.last_tid;
        self.pool.insert((addr, tid), data);
        KId(addr, Some(KTransId(vec![(tid >> 8) as u8, tid as u8])))
    }

    pub fn end(&mut self, trans: &KId) -> Option<Data> {
        if let &KId(addr, Some(KTransId(ref tid))) = trans {
            if tid.len() == 2 {
                let tid = ((tid[0] as u16) << 8) | (tid[1] as u16);
                return self.pool.remove(&(addr, tid))
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{KId, KTrans, KTransId};

    type Trans = KTrans<u32>;

    #[test]
    pub fn test_trans_mgr() {
        let mut trans = Trans::new();

        let a1 = "0.0.0.0:1234".parse().unwrap();
        let a2 = "127.0.0.1:6881".parse().unwrap();

        let t1 = trans.start(a1, 1234);
        assert_eq!(t1, KId(a1, Some(KTransId(vec![0, 1]))));
        
        let t2 = trans.start(a2, 567);
        assert_eq!(t2, KId(a2, Some(KTransId(vec![0, 2]))));
        
        let t3 = trans.start(a1, 123);
        assert_eq!(t3, KId(a1, Some(KTransId(vec![0, 3]))));

        let d1 = trans.end(&t1);
        assert_eq!(d1, Some(1234));

        let t4 = KId(a2, Some(KTransId(vec![0, 4])));
        let d4 = trans.end(&t4);
        assert_eq!(d4, None);

        let d3 = trans.end(&t3);
        assert_eq!(d3, Some(123));

        let t2 = KId(a2, Some(KTransId(vec![0, 2])));
        let d2 = trans.end(&t2);
        assert_eq!(d2, Some(567));
    }
}
