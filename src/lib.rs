mod caches;
mod models;
mod services;
mod str_utils;

use crate::caches::*;
use crate::models::*;
use crate::services::*;
use serde::Serialize;
use std::rc::Rc;
use utoipa::OpenApi;
use worker::*;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "TLT Stops API",
        version = "0.1.0",
        description = "API for Tallinn public transport stops and routes information"
    ),
    paths(
        health_check,
        get_types,
        get_routes_by_type,
        get_directions_by_route_type_number,
        get_stops_by_route_type_number_direction
    ),
    components(schemas(HealthStatus, RouteGroup, StopResponse))
)]
struct ApiDoc;

macro_rules! get_require_param {
    ($ctx:expr, $name:literal) => {{
        match $ctx.param($name) {
            Some(s) if !s.is_empty() => s,
            _ => return Response::error(concat!("missing ", $name, " query param"), 400),
        }
    }};
}

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
        .get("/api/openapi.json", openapi_spec)
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

/// Serves the OpenAPI specification
fn openapi_spec(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let openapi = ApiDoc::openapi();
    Response::from_json(&openapi)
}

#[derive(Serialize, utoipa::ToSchema)]
#[schema(example = json!({
    "status": "healthy",
    "timestamp": "2025-10-20T12:00:00Z",
    "version": "0.1.0"
}))]
struct HealthStatus {
    #[schema(example = "healthy")]
    status: &'static str,
    #[schema(example = "2025-10-20T12:00:00Z")]
    timestamp: String,
    #[schema(example = "0.1.0")]
    version: &'static str,
}

/// Health check endpoint
///
/// Returns the current status of the API service
#[utoipa::path(
    get,
    path = "/api/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthStatus)
    ),
    tag = "Health"
)]
fn health_check(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    Response::from_json(&HealthStatus {
        status: "healthy",
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Get all transport types
///
/// Returns a list of all available transport types (e.g., bus, tram, trolleybus)
#[utoipa::path(
    get,
    path = "/api/types",
    responses(
        (status = 200, description = "List of transport types", body = Vec<String>,
         example = json!(["bus", "tram", "trolleybus"]))
    ),
    tag = "Routes"
)]
async fn get_types(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let cache = Caches::get_cache();
    let from_cache = cache.types.get();
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
            cache.types.set(types.clone());
            types
        }
    };
    Response::from_json(&types)
}

/// Get routes by transport type
///
/// Returns a list of all route numbers for the specified transport type
#[utoipa::path(
    get,
    path = "/api/types/{type}/routes",
    params(
        ("type" = String, Path, description = "Transport type (e.g., bus, tram)", example = "bus")
    ),
    responses(
        (status = 200, description = "List of route numbers", body = Vec<String>,
         example = json!(["1", "2", "3"])),
        (status = 404, description = "Transport type not found")
    ),
    tag = "Routes"
)]
async fn get_routes_by_type(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let route_type = get_require_param!(ctx, "type");
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

/// Get directions for a specific route
///
/// Returns a list of all direction names for the specified route
#[utoipa::path(
    get,
    path = "/api/types/{type}/routes/{number}/directions",
    params(
        ("type" = String, Path, description = "Transport type", example = "bus"),
        ("number" = String, Path, description = "Route number", example = "1")
    ),
    responses(
        (status = 200, description = "List of direction names", body = Vec<String>,
         example = json!(["Kopli", "Linnahall"])),
        (status = 404, description = "Transport type or route not found")
    ),
    tag = "Routes"
)]
async fn get_directions_by_route_type_number(
    _req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    let route_type = get_require_param!(ctx, "type");
    let route_number = get_require_param!(ctx, "number");

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

/// Get stops for a specific route and direction
///
/// Returns a list of stop IDs and names for the specified route and direction
#[utoipa::path(
    get,
    path = "/api/types/{type}/routes/{number}/directions/{direction}/stops",
    params(
        ("type" = String, Path, description = "Transport type", example = "bus"),
        ("number" = String, Path, description = "Route number", example = "1"),
        ("direction" = String, Path, description = "Direction name (URL encoded)", example = "Kopli")
    ),
    responses(
        (status = 200, description = "List of stops with IDs and names", body = Vec<StopResponse>,
         example = json!([["1001", "Stop Name 1"], ["1002", "Stop Name 2"]])),
        (status = 400, description = "Invalid direction parameter"),
        (status = 404, description = "Transport type, route, or direction not found")
    ),
    tag = "Stops"
)]
async fn get_stops_by_route_type_number_direction(
    _req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    let route_type = get_require_param!(ctx, "type");
    let route_number = get_require_param!(ctx, "number");
    let direction_raw = get_require_param!(ctx, "direction");

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
