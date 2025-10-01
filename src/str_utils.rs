use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use memchr::memchr_iter;
use worker::Result;

use crate::models::*;

pub fn col_at_memchr_bytes(line: &[u8], target: usize) -> Option<&[u8]> {
    let mut start = 0usize;

    for (col, i) in memchr_iter(b';', line)
        .chain(std::iter::once(line.len()))
        .enumerate()
    {
        if col == target {
            return Some(&line[start..i]);
        }
        start = i.saturating_add(1);
    }
    None
}

#[derive(Default)]
pub struct LastRouteData {
    pub last_type: Option<String>,
    pub last_number: Option<String>,
}

#[inline(always)]
fn split_stops_field(stops_raw: &[u8]) -> worker::Result<Vec<String>> {
    let count = memchr_iter(b',', stops_raw).count();
    let mut stops = Vec::with_capacity(count + 1);
    let mut start = 0usize;
    for i in memchr_iter(b',', stops_raw) {
        stops.push(String::from_utf8(stops_raw[start..i].to_owned()).expect("invalid stops data"));
        start = i + 1;
    }
    if start < stops_raw.len() {
        stops.push(String::from_utf8(stops_raw[start..].to_owned()).expect("invalid stops data"));
    }
    Ok(stops)
}

pub fn extract_route_data_from_line(
    line: &[u8],
    last_data: &mut LastRouteData,
) -> Option<RouteData> {
    let mut start = 0usize;

    let mut raw_num = None;
    let mut route_num = None;
    let mut route_type = None;
    let mut direction = None;
    let mut stops = Vec::new();

    for (col, i) in memchr_iter(b';', line)
        .chain(std::iter::once(line.len()))
        .enumerate()
    {
        match col {
            0 => {
                raw_num = Some(str::from_utf8(&line[start..i]).ok()?);
                // skip validation here, just take whatever is present
            }
            3 => {
                let raw_type = Some(str::from_utf8(&line[start..i]).ok()?);
                // start validating, or fail fast if absent
                route_type = Some(
                    raw_type
                        .map(str::trim)
                        .map(str::to_string)
                        .filter(|s| !s.is_empty())
                        .or(last_data.last_type.clone())?,
                );
                last_data.last_type = route_type.clone();
                route_num = Some(
                    raw_num
                        .map(str::trim)
                        .map(str::to_string)
                        .filter(|s| !s.is_empty())
                        .or(last_data.last_number.clone())?,
                );
                last_data.last_number = route_num.clone();
            }
            10 => {
                direction = Some(str::to_string(str::trim(
                    str::from_utf8(&line[start..i]).ok()?,
                )))
                .filter(|s| !s.is_empty());
            }
            13 => {
                stops = split_stops_field(&line[start..i]).ok()?;
                break; // early exit after the last needed column
            }
            _ => {}
        }
        start = i.saturating_add(1);
    }
    Some(RouteData {
        number: route_num?.to_string(),
        route_type: route_type?.to_string(),
        directions: direction?,
        stops,
    })
}

pub async fn extract_route_data_from_buffer_fold(
    (mut buf, route_map, last_data, last_processed, first_line_skipped): (
        Vec<u8>,
        HashMap<String, HashMap<String, RouteGroup>>,
        LastRouteData,
        usize,
        bool,
    ),
    chunk: Vec<u8>,
) -> Result<(
    Vec<u8>,
    HashMap<String, HashMap<String, RouteGroup>>,
    LastRouteData,
    usize,
    bool,
)> {
    buf.extend_from_slice(&chunk);
    let (route_map, last_data, last_processed, first_line_skipped) =
        extract_route_data_from_buffer(
            &buf,
            route_map,
            last_data,
            last_processed,
            first_line_skipped,
        )
        .await?;
    Ok((
        buf,
        route_map,
        last_data,
        last_processed,
        first_line_skipped,
    ))
}

pub async fn extract_route_data_from_buffer(
    buf: &[u8],
    mut route_map: HashMap<String, HashMap<String, RouteGroup>>,
    mut last_data: LastRouteData,
    mut last_processed: usize,
    mut first_line_skipped: bool,
) -> Result<(
    HashMap<String, HashMap<String, RouteGroup>>,
    LastRouteData,
    usize,
    bool,
)> {
    let search_start = last_processed;

    for newline_pos in
        memchr::memchr_iter(b'\n', &buf[search_start..]).map(|pos| pos + search_start)
    {
        if !first_line_skipped {
            first_line_skipped = true;
            last_processed = newline_pos + 1;
            continue;
        }
        let line = &buf[last_processed..newline_pos];

        if let Some(route_data) = extract_route_data_from_line(line, &mut last_data) {
            let type_entry = route_map.entry(route_data.route_type.clone()).or_default();
            type_entry
                .entry(route_data.number.clone())
                .and_modify(|group| {
                    group
                        .directions
                        .insert(route_data.directions.clone(), route_data.stops.clone());
                })
                .or_insert({
                    let mut directions = HashMap::with_capacity(2);
                    directions.insert(route_data.directions, route_data.stops);
                    RouteGroup {
                        number: route_data.number,
                        route_type: route_data.route_type,
                        directions,
                    }
                });
        }
        last_processed = newline_pos + 1;
    }

    Ok((route_map, last_data, last_processed, first_line_skipped))
}

pub async fn extract_type_from_buffer_fold(
    (mut buf, type_set, last_processed, first_line_skipped): (
        Vec<u8>,
        HashSet<String>,
        usize,
        bool,
    ),
    chunk: Vec<u8>,
) -> Result<(Vec<u8>, HashSet<std::string::String>, usize, bool)> {
    buf.extend_from_slice(&chunk);
    let (type_set, last_processed, first_line_skipped) =
        extract_type_from_buffer(&buf, type_set, last_processed, first_line_skipped).await?;
    Ok((buf, type_set, last_processed, first_line_skipped))
}

pub async fn extract_type_from_buffer(
    buf: &[u8],
    mut type_set: HashSet<String>,
    mut last_processed: usize,
    mut first_line_skipped: bool,
) -> Result<(HashSet<std::string::String>, usize, bool)> {
    let search_start = last_processed;

    for newline_pos in
        memchr::memchr_iter(b'\n', &buf[search_start..]).map(|pos| pos + search_start)
    {
        if !first_line_skipped {
            first_line_skipped = true;
            last_processed = newline_pos + 1;
            continue;
        }
        let line = &buf[last_processed..newline_pos];

        if let Some(transport_type_bytes) = col_at_memchr_bytes(line, 3)
            && !transport_type_bytes.is_empty()
            && let Ok(transport_type) = std::str::from_utf8(transport_type_bytes)
        {
            type_set.insert(transport_type.to_owned());
        }
        last_processed = newline_pos + 1;
    }

    Ok((type_set, last_processed, first_line_skipped))
}

pub fn extract_stop_data_from_line(
    line: &[u8],
    last_name: &Option<Rc<String>>,
) -> Option<Rc<StopData>> {
    let mut start = 0usize;

    let mut id = None;
    let mut siri_id = None;
    let mut name = None;

    for (col, i) in memchr_iter(b';', line)
        .chain(std::iter::once(line.len()))
        .enumerate()
    {
        match col {
            0 => {
                id = Some(str::from_utf8(&line[start..i]).ok()?);
            }
            1 => {
                siri_id = Some(str::from_utf8(&line[start..i]).ok()?);
            }
            5 => {
                name = Some(str::from_utf8(&line[start..i]).ok()?);
                break; // early exit after the last needed column
            }
            _ => {}
        }

        start = i.saturating_add(1);
    }

    let name = name
        .map(str::trim)
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .map(Rc::new)
        .or_else(|| last_name.as_ref().map(|name| Rc::clone(&name)))?;
    let siri_id = siri_id
        .map(str::trim)
        .map(str::to_string)
        .filter(|s| !s.is_empty())?;
    let id = id
        .map(str::trim)
        .map(str::to_string)
        .filter(|s| !s.is_empty())?;

    Some(Rc::new(StopData {
        id: id,
        siri_id: siri_id,
        name: name,
    }))
}

pub async fn extract_stop_data_from_buffer_fold(
    (mut buf, stop_map, last_name, last_processed, first_line_skipped): (
        Vec<u8>,
        HashMap<String, Rc<StopData>>,
        Option<Rc<String>>,
        usize,
        bool,
    ),
    chunk: Vec<u8>,
) -> Result<(
    Vec<u8>,
    HashMap<String, Rc<StopData>>,
    Option<Rc<String>>,
    usize,
    bool,
)> {
    buf.extend_from_slice(&chunk);
    let (stop_map, last_name, last_processed, first_line_skipped) = extract_stop_data_from_buffer(
        &buf,
        stop_map,
        last_name,
        last_processed,
        first_line_skipped,
    )
    .await?;
    Ok((buf, stop_map, last_name, last_processed, first_line_skipped))
}

pub async fn extract_stop_data_from_buffer(
    buf: &[u8],
    mut stop_map: HashMap<String, Rc<StopData>>,
    mut last_name: Option<Rc<String>>,
    mut last_processed: usize,
    mut first_line_skipped: bool,
) -> Result<(
    HashMap<String, Rc<StopData>>,
    Option<Rc<String>>,
    usize,
    bool,
)> {
    let search_start = last_processed;

    for newline_pos in
        memchr::memchr_iter(b'\n', &buf[search_start..]).map(|pos| pos + search_start)
    {
        if !first_line_skipped {
            first_line_skipped = true;
            last_processed = newline_pos + 1;
            continue;
        }
        let line = &buf[last_processed..newline_pos];

        if let Some(stop_data) = extract_stop_data_from_line(line, &last_name) {
            last_name = Some(Rc::clone(&stop_data.name));
            stop_map.insert(stop_data.id.clone(), Rc::clone(&stop_data));
            stop_map.insert(stop_data.siri_id.clone(), stop_data);
        }
        last_processed = newline_pos + 1;
    }

    Ok((stop_map, last_name, last_processed, first_line_skipped))
}
