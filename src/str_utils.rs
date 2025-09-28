use std::collections::{HashMap, HashSet};

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
pub struct LastData {
    pub last_type: Option<String>,
    pub last_number: Option<String>,
}

pub fn extract_route_data_from_line(line: &[u8], last_data: &mut LastData) -> Option<RouteData> {
    let mut start = 0usize;

    let mut raw_num = None;
    let mut route_num = None;
    let mut route_type = None;
    let mut direction = None;

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
    })
}

pub async fn extract_route_data_from_buffer_fold(
    (mut buf, route_map, last_data, last_processed, first_line_skipped): (
        Vec<u8>,
        HashMap<String, HashMap<String, RouteGroup>>,
        LastData,
        usize,
        bool,
    ),
    chunk: Vec<u8>,
) -> Result<(
    Vec<u8>,
    HashMap<String, HashMap<String, RouteGroup>>,
    LastData,
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
    mut last_data: LastData,
    mut last_processed: usize,
    mut first_line_skipped: bool,
) -> Result<(
    HashMap<String, HashMap<String, RouteGroup>>,
    LastData,
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
                    group.directions.push(route_data.directions.clone());
                })
                .or_insert({
                    let mut directions = Vec::with_capacity(2);
                    directions.push(route_data.directions.clone());
                    RouteGroup {
                        number: route_data.number.clone(),
                        route_type: route_data.route_type.clone(),
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
