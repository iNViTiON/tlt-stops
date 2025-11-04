mod caches;
mod models;
mod services;
mod str_utils;

use crate::caches::*;
use crate::models::*;
use crate::services::*;
use crate::str_utils::splits_commas;
use serde::Serialize;
use std::collections::HashSet;
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
        get_stops_by_route_type_number_direction,
        get_stop_arrivals,
    ),
    components(schemas(HealthStatus, StopResponse, PostArrivalsResponse, StopArrivals, StopArrival, Arrival))
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
    fn from(error: ParsingUpstreamError) -> Self {
        match error {
            ParsingUpstreamError::Http(e) => worker::Error::Json((e.to_string(), 502)),
            ParsingUpstreamError::Utf8 => {
                worker::Error::RustError("UTF-8 parsing error".to_string())
            }
            ParsingUpstreamError::Error(msg) => worker::Error::Json((msg, 500)),
        }
    }
}

impl From<RequestError> for worker::Error {
    fn from(error: RequestError) -> Self {
        match error {
            RequestError::MissingParameter(msg) | RequestError::InvalidParameter(msg) => {
                worker::Error::Json((msg, 400))
            }
        }
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
        .get_async("/api/arrivals", get_stop_arrivals)
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
            .get_stop_name_by_id_async(stop_id)
            .await
            .unwrap_or_else(|| Rc::new("Can't resolve stop name".to_string()));
        stops_data.push((stop_id, stop_name));
    }

    Response::from_json(&stops_data)
}

/// Get arrival times for specific stops
///
/// Returns real-time arrival information for the requested stops
#[utoipa::path(
    get,
    path = "/api/arrivals",
    params(
        ("stops" = String, Query, description = "Comma-separated list of stop IDs (max 5)", example = "1001,1002,1003"),
    ),
    responses(
        (status = 200, description = "Arrival times for requested stops", body = PostArrivalsResponse),
        (status = 400, description = "Invalid request - no stops provided or too many stops (max 5)"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Arrivals"
)]
async fn get_stop_arrivals(req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let stops_param = req.url()?;
    let stops_param = stops_param
        .query_pairs()
        .find_map(|(k, v)| {
            if k == "stops" && !v.is_empty() {
                return Some(v);
            }
            None
        })
        .ok_or(RequestError::MissingParameter(String::from(
            "missing stops query parameter",
        )))?;
    let stops_request = splits_commas(stops_param.as_bytes()).map_err(|_| {
        RequestError::InvalidParameter(String::from("invalid stops query parameter"))
    })?;
    {
        let stop_count = stops_request.len();
        if !(1..=5).contains(&stop_count) {
            return Response::error("invalid number of stops provided (1-5)", 400);
        }
    }
    let service = TransportService::get_service();
    let stop_map = service.get_stop_map().await?;
    let arrivals_cache = &Caches::get_cache().stop_arrival;
    let mut stop_states: Vec<StopArrivalState> = stops_request
        .into_iter()
        .map(StopId)
        .map(StopArrivalState::StopId)
        .map(|state| match state {
            StopArrivalState::StopId(stop_id) => stop_id.validate(&stop_map),
            other => other,
        })
        .map(|state| match state {
            StopArrivalState::Valid(valid_stop_id) => {
                valid_stop_id.fetch_arrivals_from_cache(arrivals_cache)
            }
            other => other,
        })
        .collect();
    let missing_caches = stop_states
        .iter()
        .filter_map(|state| match state {
            StopArrivalState::Valid(stop) => Some(stop.data.siri_id.to_string()),
            _ => None,
        })
        .collect::<HashSet<String>>()
        .into_iter()
        .fold(String::new(), |mut acc, id| {
            if !acc.is_empty() {
                acc.push(',');
            }
            acc.push_str(&id);
            acc
        });
    if !missing_caches.is_empty() {
        service.update_stops_arrival_cache(&missing_caches).await?;
        stop_states = stop_states
            .into_iter()
            .map(|state| match state {
                StopArrivalState::Valid(valid_stop_id) => {
                    valid_stop_id.fetch_arrivals_from_cache(arrivals_cache)
                }
                other => other,
            })
            .collect();
    }
    let stop_arrivals = stop_states
        .into_iter()
        .map(|state| match state {
            StopArrivalState::Ready(ready_stop_arrivals) => Ok(Some(ready_stop_arrivals.0)),
            StopArrivalState::Invalid => Ok(None),
            _ => Err(ParsingUpstreamError::Error(String::from(
                "unexpected state when fetching arrivals from cache",
            ))),
        })
        .collect::<core::result::Result<Vec<Option<Rc<StopArrivals>>>, ParsingUpstreamError>>()
        .map(|stops| PostArrivalsResponse { stops });
    Response::from_json(&stop_arrivals?)
}
