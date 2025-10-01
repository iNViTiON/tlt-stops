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
    data: T,
    time: u32,
}

pub struct CacheData<T> {
    record: Option<CacheRecord<T>>,
    ttl_secs: u32,
}
impl<T> CacheData<T> {
    pub fn new(ttl_secs: u32) -> Self {
        CacheData::<T> {
            record: None,
            ttl_secs,
        }
    }

    pub fn set(&mut self, data: T) {
        self.record = Some(CacheRecord {
            data,
            time: now_secs(),
        });
    }

    pub fn get(&mut self) -> Option<&T> {
        if let Some(record) = self.record.as_ref()
            && now_secs() - record.time > self.ttl_secs
        {
            self.record = None;
        }
        self.record.as_ref().map(|record| &record.data)
    }
}

pub struct Caches {
    routes_raw: RefCell<CacheData<Rc<Vec<u8>>>>,
    stop_map: RefCell<CacheData<Rc<HashMap<String, Rc<StopData>>>>>,
    stops_raw: RefCell<CacheData<Rc<Vec<u8>>>>,
    types: RefCell<CacheData<Rc<Vec<String>>>>,
}
impl Caches {
    pub fn get_cache() -> &'static SendWrapper<Caches> {
        CACHE.get_or_init(|| SendWrapper::new(Caches::new()))
    }

    pub fn new() -> Self {
        let routes_raw = RefCell::new(CacheData::new(60 * 60 * 3));
        let stop_map = RefCell::new(CacheData::new(60 * 60 * 3));
        let stops_raw = RefCell::new(CacheData::new(60 * 60 * 3));
        let types = RefCell::new(CacheData::new(60 * 60 * 24));
        Caches {
            routes_raw,
            stop_map,
            stops_raw,
            types,
        }
    }

    pub fn set_routes(&self, data: Rc<Vec<u8>>) {
        self.routes_raw.borrow_mut().set(data);
    }

    pub fn get_routes(&self) -> Option<Rc<Vec<u8>>> {
        self.routes_raw.borrow_mut().get().cloned()
    }

    pub fn set_stop_map(&self, data: Rc<HashMap<String, Rc<StopData>>>) {
        self.stop_map.borrow_mut().set(data);
    }

    pub fn get_stop_map(&self) -> Option<Rc<HashMap<String, Rc<StopData>>>> {
        self.stop_map.borrow_mut().get().cloned()
    }

    pub fn set_stops(&self, data: Rc<Vec<u8>>) {
        self.stops_raw.borrow_mut().set(data);
    }

    pub fn get_stops(&self) -> Option<Rc<Vec<u8>>> {
        self.stops_raw.borrow_mut().get().cloned()
    }

    pub fn set_types(&self, data: Rc<Vec<String>>) {
        self.types.borrow_mut().set(data);
    }

    pub fn get_types(&self) -> Option<Rc<Vec<String>>> {
        self.types.borrow_mut().get().cloned()
    }
}
