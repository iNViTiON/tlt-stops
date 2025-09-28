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
            let mut routes = routes.keys()
                .collect::<Vec<&String>>();
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
                Response::from_json(&route.directions)
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
