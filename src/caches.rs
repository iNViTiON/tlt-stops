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

    pub fn set(&self, data: Rc<T>) {
        let expires_at = now_secs().saturating_add(self.ttl_secs);
        self.record
            .borrow_mut()
            .replace(CacheRecord { data, expires_at });
    }

    fn purge_expired(&self) {
        if let Some(record) = self.record.borrow().as_ref()
            && now_secs() > record.expires_at
        {
            self.record.take();
        }
    }
    pub fn get(&self) -> Option<Rc<T>> {
        self.purge_expired();
        self.record
            .borrow()
            .as_ref()
            .map(|record| Rc::clone(&record.data))
    }
}

struct CacheDataWithKeys<K, T> {
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

    pub fn set(&self, key: K, data: Rc<T>) {
        let expires_at = now_secs().saturating_add(self.ttl_secs);
        self.record
            .borrow_mut()
            .insert(key, CacheRecord { data, expires_at });
    }

    fn purge_expired(&self) {
        let now = now_secs();
        self.record
            .borrow_mut()
            .retain(|_, record| record.expires_at > now);
    }
    pub fn get(&self, key: &K) -> Option<Rc<T>> {
        self.purge_expired();
        self.record
            .borrow()
            .get(key)
            .map(|record| Rc::clone(&record.data))
    }
}

pub struct Caches {
    pub routes_raw: CacheData<Vec<u8>>,
    pub stop_map: CacheData<HashMap<String, Rc<StopData>>>,
    pub stops_raw: CacheData<Vec<u8>>,
    pub types: CacheData<Vec<String>>,
    // arrivals: RefCell<HashMap
}
impl Caches {
    pub fn get_cache() -> &'static SendWrapper<Caches> {
        CACHE.get_or_init(|| SendWrapper::new(Caches::new()))
    }

    pub fn new() -> Self {
        let routes_raw = CacheData::new(60 * 60 * 3);
        let stop_map = CacheData::new(60 * 60 * 3);
        let stops_raw = CacheData::new(60 * 60 * 3);
        let types = CacheData::new(60 * 60 * 24);
        Self {
            routes_raw,
            stop_map,
            stops_raw,
            types,
        }
    }
}
