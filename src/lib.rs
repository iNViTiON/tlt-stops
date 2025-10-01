mod caches;
mod models;
mod services;
mod str_utils;

use crate::caches::*;
use crate::services::*;
use serde::Serialize;
use std::rc::Rc;
use worker::*;

pub enum HttpResponseError {
    Worker(worker::Error),
    Upstream(ParsingUpstreamError),
}
impl From<worker::Error> for HttpResponseError {
    fn from(error: worker::Error) -> Self {
        HttpResponseError::Worker(error)
    }
}
impl From<ParsingUpstreamError> for HttpResponseError {
    fn from(error: ParsingUpstreamError) -> Self {
        HttpResponseError::Upstream(error)
    }
}
impl From<ParsingUpstreamError> for worker::Error {
    fn from(_: ParsingUpstreamError) -> Self {
        worker::Error::BadEncoding
    }
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    Router::new()
        .get("/api/health", health_check)
        .get_async("/api/types", get_types)
        .get_async("/api/types/:type/routes", get_routes_by_type)
        .get_async(
            "/api/types/:type/routes/:number/directions",
            get_directions_by_route_type_number,
        )
        .get_async(
            "/api/types/:type/routes/:number/directions/:direction/stops",
            get_stops_by_route_type_number_direction,
        )
        .run(req, env)
        .await
}

#[derive(Serialize)]
struct HealthStatus {
    status: &'static str,
    timestamp: String,
    version: &'static str,
}

fn health_check(_req: Request, _ctx: RouteContext<()>) -> Result<worker::Response> {
    Response::from_json(&HealthStatus {
        status: "healthy",
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn get_types(_req: Request, _ctx: RouteContext<()>) -> Result<worker::Response> {
    let cache = Caches::get_cache();
    let from_cache = cache.get_types();
    let types = match from_cache {
        Some(types) => types,
        None => {
            let service = TransportService::get_service();
            let mut types = service
                .get_types()
                .await?
                .into_iter()
                .collect::<Vec<String>>();
            types.sort_unstable();
            let types = Rc::new(types);
            cache.set_types(types.clone());
            types
        }
    };
    Response::from_json(&types)
}

async fn get_routes_by_type(_req: Request, ctx: RouteContext<()>) -> Result<worker::Response> {
    let route_type = match ctx.param("type").filter(|s| !s.is_empty()) {
        Some(route_type) => route_type,
        _ => return Response::error("missing type query param", 400),
    };
    let service = TransportService::get_service();
    let route_map = service.get_route_map().await?;
    let routes = route_map.get(route_type);
    match routes {
        Some(routes) => {
            let mut routes = routes.keys().collect::<Vec<&String>>();
            routes.sort_unstable();
            Response::from_json(&routes)
        }
        None => Response::error("type not found", 404),
    }
}

async fn get_directions_by_route_type_number(
    _req: Request,
    ctx: RouteContext<()>,
) -> Result<worker::Response> {
    let (route_type, route_number) = match (
        ctx.param("type").filter(|s| !s.is_empty()),
        ctx.param("number").filter(|s| !s.is_empty()),
    ) {
        (Some(route_type), Some(route_number)) => (route_type, route_number),
        _ => return Response::error("missing type/number query param", 400),
    };

    let service = TransportService::get_service();
    let route_map = service.get_route_map().await?;

    let routes = match route_map.get(route_type) {
        Some(routes) => routes,
        None => return Response::error("type not found", 404),
    };

    let route = match routes.get(route_number) {
        Some(route) => route,
        None => return Response::error("route number not found", 404),
    };

    let mut directions: Vec<&str> = route.directions.keys().map(|s| s.as_str()).collect();
    directions.sort_unstable();

    Response::from_json(&directions)
}

async fn get_stops_by_route_type_number_direction(
    _req: Request,
    ctx: RouteContext<()>,
) -> Result<worker::Response> {
    let (route_type, route_number, direction_raw) = match (
        ctx.param("type").filter(|s| !s.is_empty()),
        ctx.param("number").filter(|s| !s.is_empty()),
        ctx.param("direction").filter(|s| !s.is_empty()),
    ) {
        (Some(route_type), Some(route_number), Some(direction_raw)) => {
            (route_type, route_number, direction_raw)
        }
        _ => return Response::error("missing type/number/direction query param", 400),
    };

    let service = TransportService::get_service();
    let route_map = service.get_route_map().await?;

    let routes = match route_map.get(route_type) {
        Some(routes) => routes,
        None => return Response::error("type not found", 404),
    };

    let route = match routes.get(route_number) {
        Some(route) => route,
        None => return Response::error("route number not found", 404),
    };

    let direction = match urlencoding::decode(direction_raw) {
        Ok(direction) if !direction.is_empty() => direction.to_string(),
        _ => return Response::error("invalid direction", 400),
    };

    let stops = match route.directions.get(&direction) {
        Some(stops) => stops,
        None => return Response::error("direction not found", 404),
    };

    let mut stops_data = Vec::with_capacity(stops.len());
    for stop_id in stops {
        let stop_name = service
            .get_stop_name_by_id(stop_id)
            .await
            .unwrap_or_else(|| Rc::new("Can't resolve stop name".to_string()));
        stops_data.push((stop_id, stop_name));
    }

    Response::from_json(&stops_data)
}
