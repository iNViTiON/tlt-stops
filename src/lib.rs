mod caches;
mod models;
mod services;
mod str_utils;

use crate::caches::*;
use crate::services::*;
use futures::future;
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
    let route_type = ctx.param("type");
    if let Some(route_type) = route_type
        && !route_type.is_empty()
    {
        let service = TransportService::get_service();
        let route_map = service.get_route_map().await?;
        let routes = route_map.get(route_type);
        if let Some(routes) = routes {
            let mut routes = routes.keys().collect::<Vec<&String>>();
            routes.sort_unstable();
            Response::from_json(&routes)
        } else {
            Response::error("type not found", 404)
        }
    } else {
        Response::error("missing type query param", 400)
    }
}

async fn get_directions_by_route_type_number(
    _req: Request,
    ctx: RouteContext<()>,
) -> Result<worker::Response> {
    let route_type = ctx.param("type");
    let route_number = ctx.param("number");

    if let Some(route_type) = route_type
        && !route_type.is_empty()
        && let Some(route_number) = route_number
        && !route_number.is_empty()
    {
        let service = TransportService::get_service();
        let route_map = service.get_route_map().await?;
        let routes = route_map.get(route_type);
        if let Some(routes) = routes {
            let route = routes.get(route_number);
            if let Some(route) = route {
                let mut directions = route.directions.keys().collect::<Vec<&String>>();
                directions.sort_unstable();
                Response::from_json(&directions)
            } else {
                Response::error("route number not found", 404)
            }
        } else {
            Response::error("type not found", 404)
        }
    } else {
        Response::error("missing type/number query param", 400)
    }
}

async fn get_stops_by_route_type_number_direction(
    _req: Request,
    ctx: RouteContext<()>,
) -> Result<worker::Response> {
    let route_type = ctx.param("type");
    let route_number = ctx.param("number");
    let direction = ctx.param("direction");

    if let Some(route_type) = route_type
        && !route_type.is_empty()
        && let Some(route_number) = route_number
        && !route_number.is_empty()
        && let Some(direction) = direction
        && !direction.is_empty()
    {
        let service = TransportService::get_service();
        let route_map = service.get_route_map().await?;
        let routes = route_map.get(route_type);
        if let Some(routes) = routes {
            let route = routes.get(route_number);
            if let Some(route) = route {
                let direction = urlencoding::decode(direction).expect("can't decode direction");
                let stops = route.directions.get(&*direction);
                if let Some(stops) = stops {
                    let stop_names =
                        future::join_all(stops.iter().map(|id| service.get_stop_name_by_id(id)))
                            .await
                            .into_iter()
                            .map(|name| name.expect("cannot get stop name"));
                    let stops: Vec<(&String, Rc<String>)> =
                        stops.into_iter().zip(stop_names).collect();
                    Response::from_json(&stops)
                } else {
                    Response::error("direction not found", 404)
                }
            } else {
                Response::error("route number not found", 404)
            }
        } else {
            Response::error("type not found", 404)
        }
    } else {
        Response::error("missing type/number/direction query param", 400)
    }
}
