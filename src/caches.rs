use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::OnceLock;
use worker::send::SendWrapper;

use crate::models::*;

pub static CACHE: OnceLock<SendWrapper<Caches>> = OnceLock::new();

fn now_secs() -> u32 {
    (js_sys::Date::now() / 1000.0) as u32
}

struct CacheRecord<T> {
    data: Rc<T>,
    expires_at: u32,
}

pub struct CacheData<T> {
    record: RefCell<Option<CacheRecord<T>>>,
    ttl_secs: u32,
}
impl<T> CacheData<T> {
    pub fn new(ttl_secs: u32) -> Self {
        CacheData::<T> {
            record: RefCell::new(None),
            ttl_secs,
        }
    }

    pub fn set(&self, data: Rc<T>) -> Result<(), ()> {
        let expires_at = now_secs().saturating_add(self.ttl_secs);
        let mut record = self.record.try_borrow_mut().map_err(|_| ())?;
        record.replace(CacheRecord { data, expires_at });
        Ok(())
    }

    pub fn get(&self) -> Option<Rc<T>> {
        let record = self.record.try_borrow().ok()?;
        let record = (*record).as_ref()?;
        if now_secs() > record.expires_at {
            drop(record);
            let _ = self.record.try_borrow_mut().ok().map(|mut rec| rec.take());
            None
        } else {
            Some(Rc::clone(&record.data))
        }
    }
}

pub struct CacheDataWithKeys<K, T> {
    record: RefCell<HashMap<K, CacheRecord<T>>>,
    ttl_secs: u32,
}
impl<K, T> CacheDataWithKeys<K, T>
where
    K: std::hash::Hash + Eq + Clone,
{
    pub fn new(ttl_secs: u32) -> Self {
        CacheDataWithKeys::<K, T> {
            record: RefCell::new(HashMap::new()),
            ttl_secs,
        }
    }

    pub fn set(&self, key: K, data: Rc<T>) -> Result<(), ()> {
        let expires_at = now_secs().saturating_add(self.ttl_secs);
        let mut record = self.record.try_borrow_mut().map_err(|_| ())?;
        record.insert(key, CacheRecord { data, expires_at });
        Ok(())
    }

    pub fn get(&self, key: &K) -> Option<Rc<T>> {
        let record = self.record.try_borrow().ok()?;
        let record = record.get(key)?;
        if now_secs() > record.expires_at {
            drop(record);
            let _ = self
                .record
                .try_borrow_mut()
                .ok()
                .map(|mut rec| rec.remove(key));
            None
        } else {
            Some(Rc::clone(&record.data))
        }
    }
}

pub struct Caches {
    pub routes_raw: CacheData<Vec<u8>>,
    pub stop_arrival: CacheDataWithKeys<String, StopArrivals>,
    pub stop_map: CacheData<HashMap<String, Rc<StopData>>>,
    pub stops_raw: CacheData<Vec<u8>>,
    pub types: CacheData<Vec<String>>,
}
impl Caches {
    pub fn get_cache() -> &'static SendWrapper<Caches> {
        CACHE.get_or_init(|| SendWrapper::new(Caches::new()))
    }

    pub fn new() -> Self {
        let routes_raw = CacheData::new(60 * 60 * 3);
        let stop_arrival = CacheDataWithKeys::new(10);
        let stop_map = CacheData::new(60 * 60 * 3);
        let stops_raw = CacheData::new(60 * 60 * 3);
        let types = CacheData::new(60 * 60 * 24);
        Self {
            routes_raw,
            stop_arrival,
            stop_map,
            stops_raw,
            types,
        }
    }
}
