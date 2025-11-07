use crate::Caches;
use crate::models::*;
use crate::str_utils::*;

use futures::TryStreamExt;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::sync::OnceLock;
use worker::ByteStream;
use worker::send::SendWrapper;

pub static SERVICE: OnceLock<SendWrapper<TransportService>> = OnceLock::new();

#[derive(Debug)]
pub enum ParsingUpstreamError {
    Http(worker::Error),
    Utf8,
    Error(String),
}

impl From<worker::Error> for ParsingUpstreamError {
    fn from(err: worker::Error) -> Self {
        ParsingUpstreamError::Http(err)
    }
}

impl From<FromUtf8Error> for ParsingUpstreamError {
    fn from(_err: FromUtf8Error) -> Self {
        ParsingUpstreamError::Utf8
    }
}

impl From<Utf8Error> for ParsingUpstreamError {
    fn from(_err: Utf8Error) -> Self {
        ParsingUpstreamError::Utf8
    }
}

pub struct TransportService {}

impl TransportService {
    pub fn get_service() -> &'static SendWrapper<TransportService> {
        SERVICE.get_or_init(|| SendWrapper::new(TransportService::new()))
    }

    pub fn new() -> Self {
        Self {}
    }

    async fn get_routes_stream() -> worker::Result<ByteStream> {
        let uri = "https://transport.tallinn.ee/data/routes.txt";
        let req_init = worker::RequestInit {
            method: worker::Method::Get,
            cf: worker::CfProperties {
                cache_ttl: Some(3600),
                ..Default::default()
            },
            ..Default::default()
        };
        let req = worker::Request::new_with_init(uri, &req_init)?;
        let mut res = worker::Fetch::Request(req).send().await?;
        res.stream()
    }

    async fn get_stops_stream() -> worker::Result<ByteStream> {
        let uri = "https://transport.tallinn.ee/data/stops.txt";
        let req_init = worker::RequestInit {
            method: worker::Method::Get,
            cf: worker::CfProperties {
                cache_ttl: Some(3600),
                ..Default::default()
            },
            ..Default::default()
        };
        let req = worker::Request::new_with_init(uri, &req_init)?;
        let mut res = worker::Fetch::Request(req).send().await?;
        res.stream()
    }

    async fn get_stops_arrivals(&self, stop_siri_ids: &str) -> worker::Result<String> {
        let uri = format!(
            "https://transport.tallinn.ee/siri-stop-departures.php?stopid={}",
            stop_siri_ids
        );
        let req_init = worker::RequestInit {
            method: worker::Method::Get,
            cf: worker::CfProperties {
                cache_ttl: Some(120),
                ..Default::default()
            },
            ..Default::default()
        };
        let req = worker::Request::new_with_init(&uri, &req_init)?;
        let mut res = worker::Fetch::Request(req).send().await?;
        res.text().await
    }

    pub async fn update_stops_arrival_cache(
        &self,
        stop_siri_ids: &str,
    ) -> core::result::Result<(), ParsingUpstreamError> {
        if stop_siri_ids.is_empty() {
            return Ok(());
        }
        let arrivals_raw = self.get_stops_arrivals(stop_siri_ids).await?;
        let arrivals_bytes = arrivals_raw.as_bytes();
        let stop_map = self.get_stop_map().await?;
        let cache = Caches::get_cache();
        let stop_arrival_cache = &cache.stop_arrival;
        let stop_arrivals = split_arrival_by_stops(arrivals_bytes).flat_map(|stop_arrival_raw| {
            self::extract_arrival_stop_data_from_line(stop_arrival_raw, &stop_map)
        });
        for stop_arrival in stop_arrivals {
            let stop_arrival = stop_arrival?;
            let stop_arrival = Rc::new(stop_arrival);
            stop_arrival_cache.set(stop_arrival.id.clone(), stop_arrival);
        }
        Ok(())
    }

    pub async fn get_types(&self) -> Result<HashSet<String>, ParsingUpstreamError> {
        let cache = Caches::get_cache();
        let from_cache = cache.routes_raw.get();

        let (buf, type_set) = match from_cache {
            Some(cache) => {
                let (type_set, _, _) =
                    extract_type_from_buffer(&cache[..], HashSet::with_capacity(5), 0usize, false)
                        .await?;
                (None, type_set)
            }
            None => {
                let reader = Self::get_routes_stream().await?;
                let (mut buf, type_set, _, _) = reader
                    .try_fold(
                        (
                            Vec::with_capacity(128 * 1024),
                            HashSet::with_capacity(5),
                            0usize,
                            false,
                        ),
                        extract_type_from_buffer_fold,
                    )
                    .await?;
                buf.shrink_to_fit();
                (Some(buf), type_set)
            }
        };

        if let Some(buf) = buf {
            cache.routes_raw.set(Rc::new(buf));
        }

        Ok(type_set)
    }

    pub async fn get_route_map(
        &self,
    ) -> Result<HashMap<String, HashMap<String, RouteGroup>>, ParsingUpstreamError> {
        let cache = Caches::get_cache();
        let from_cache = cache.routes_raw.get();

        let (buf, route_map) = match from_cache {
            Some(cache) => {
                let (route_map, _, _, _) = extract_route_data_from_buffer(
                    &cache[..],
                    HashMap::new(),
                    LastRouteData::default(),
                    0usize,
                    false,
                )
                .await?;
                (None, route_map)
            }
            None => {
                let reader = Self::get_routes_stream().await?;
                let (mut buf, route_map, _, _, _) = reader
                    .try_fold(
                        (
                            Vec::with_capacity(128 * 1024),
                            HashMap::<String, HashMap<String, RouteGroup>>::new(),
                            LastRouteData::default(),
                            0usize,
                            false,
                        ),
                        extract_route_data_from_buffer_fold,
                    )
                    .await?;
                buf.shrink_to_fit();
                (Some(buf), route_map)
            }
        };

        if let Some(buf) = buf {
            cache.routes_raw.set(Rc::new(buf));
        }

        Ok(route_map)
    }

    pub async fn get_stop_map(
        &self,
    ) -> Result<Rc<HashMap<String, Rc<StopData>>>, ParsingUpstreamError> {
        let cache = Caches::get_cache();

        let from_cache = cache.stop_map.get();
        if let Some(stop_map) = from_cache {
            return Ok(stop_map);
        }

        let from_cache = cache.stops_raw.get();

        let (buf, stop_map) = match from_cache {
            Some(cache) => {
                let (stop_map, _, _, _) =
                    extract_stop_data_from_buffer(&cache[..], HashMap::new(), None, 0usize, false)
                        .await?;
                (None, stop_map)
            }
            None => {
                let reader = Self::get_stops_stream().await?;
                let (mut buf, stop_map, _, _, _) = reader
                    .try_fold(
                        (
                            Vec::with_capacity(90 * 1024),
                            HashMap::<String, Rc<StopData>>::new(),
                            None,
                            0usize,
                            false,
                        ),
                        extract_stop_data_from_buffer_fold,
                    )
                    .await?;
                buf.shrink_to_fit();
                (Some(buf), stop_map)
            }
        };

        if let Some(buf) = buf {
            cache.stops_raw.set(Rc::new(buf));
        }

        let stop_map = Rc::new(stop_map);
        cache.stop_map.set(Rc::clone(&stop_map));

        Ok(stop_map)
    }

    #[inline(always)]
    pub async fn get_stop_name_by_id_async(&self, stop_id: &str) -> Option<Rc<String>> {
        let stop_map = self.get_stop_map().await.ok()?;
        TransportService::get_stop_name_by_id(stop_id, &stop_map)
    }

    #[inline(always)]
    pub fn get_stop_name_by_id(
        stop_id: &str,
        stop_map: &HashMap<String, Rc<StopData>>,
    ) -> Option<Rc<String>> {
        stop_map
            .get(stop_id)
            .map(|stop_data| Rc::clone(&stop_data.name))
    }
}
