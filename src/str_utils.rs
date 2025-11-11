use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::string::FromUtf8Error;

use chrono::offset::LocalResult;
use chrono::{NaiveDateTime, NaiveTime, TimeZone, Utc};
use memchr::{memchr_iter, memmem};
use worker::Result;

use crate::models::*;
use crate::services::*;

pub fn seconds_from_midnight_to_utc_iso(
    seconds_from_midnight: u32,
) -> core::result::Result<String, &'static str> {
    use chrono_tz::Europe::Tallinn;
    let is_next_day = seconds_from_midnight >= 86400;
    let seconds_from_midnight = if is_next_day {
        seconds_from_midnight - 86400
    } else {
        seconds_from_midnight
    };
    let time = NaiveTime::from_num_seconds_from_midnight_opt(seconds_from_midnight, 0)
        .ok_or("seconds_from_midnight must be in 0..=86399")?;

    let today_tallinn = Utc::now().with_timezone(&Tallinn).date_naive();
    let mut naive_dt = NaiveDateTime::new(today_tallinn, time);
    if is_next_day {
        naive_dt = naive_dt
            .checked_add_days(chrono::Days::new(1))
            .ok_or("date overflow")?;
    }

    match Tallinn.from_local_datetime(&naive_dt) {
        LocalResult::Single(dt_tallinn) => Ok(dt_tallinn.with_timezone(&Utc).to_rfc3339()),
        // If local time is ambiguous (fall-back), pick the earlier occurrence.
        LocalResult::Ambiguous(earliest, _latest) => Ok(earliest.with_timezone(&Utc).to_rfc3339()),
        // If local time doesn't exist (spring-forward gap), surface an error.
        LocalResult::None => Err("Local time does not exist in Tallinn today (DST gap)"),
    }
}

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

#[inline(always)]
pub fn splits_commas(input: &[u8]) -> core::result::Result<Vec<String>, FromUtf8Error> {
    let count = memchr_iter(b',', input).count();
    let mut parts = Vec::with_capacity(count + 1);
    let mut start = 0usize;
    for i in memchr_iter(b',', input).chain(std::iter::once(input.len())) {
        parts.push(String::from_utf8(input[start..i].to_owned())?);
        start = i + 1;
    }
    Ok(parts)
}

#[derive(Default)]
pub struct LastRouteData {
    pub last_type: Option<String>,
    pub last_number: Option<String>,
}

#[inline(always)]
fn split_stops_field(stops_raw: &[u8]) -> core::result::Result<Vec<String>, ParsingUpstreamError> {
    let count = memchr_iter(b',', stops_raw).count();
    let mut stops = Vec::with_capacity(count + 1);
    let mut start = 0usize;
    for i in memchr_iter(b',', stops_raw) {
        stops.push(String::from_utf8(stops_raw[start..i].to_owned())?);
        start = i + 1;
    }
    if start < stops_raw.len() {
        stops.push(String::from_utf8(stops_raw[start..].to_owned())?);
    }
    Ok(stops)
}

pub fn split_arrival_by_stops(arrival: &[u8]) -> impl Iterator<Item = &[u8]> {
    let mut start = 0usize;
    memmem::find_iter(arrival, b"\nstop,")
        .chain(std::iter::once(arrival.len()))
        .map(move |i| {
            let part = &arrival[start..i];
            start = i + 1;
            part
        })
        .skip(1)
}

pub fn remove_trailing_newline(input: &[u8]) -> &[u8] {
    if let Some(last_byte) = input.last()
        && *last_byte == b'\n'
    {
        &input[..input.len() - 1]
    } else {
        input
    }
}

pub fn extract_arrival_data(
    arrival_line: &[u8],
) -> core::result::Result<StopArrival, ParsingUpstreamError> {
    let mut start = 0usize;

    let mut route_number = None;
    let mut route_type = None;
    let mut expected_time = None;
    let mut arrival_type = None;

    for (col, i) in memchr_iter(b',', arrival_line)
        .chain(std::iter::once(arrival_line.len()))
        .enumerate()
    {
        let current = unsafe { str::from_utf8_unchecked(&arrival_line[start..i]) };
        match col {
            0 => {
                route_type = Some(current);
            }
            1 => {
                route_number = Some(current);
            }
            2 => {
                expected_time = Some(
                    seconds_from_midnight_to_utc_iso(
                        current
                            .parse::<u32>()
                            .map_err(|_| ParsingUpstreamError::Utf8)?,
                    )
                    .map_err(|_| ParsingUpstreamError::Utf8)?,
                );
            }
            6 => {
                let expected_time = expected_time.ok_or(ParsingUpstreamError::Error(
                    String::from("incorrect arrival time"),
                ))?;
                arrival_type = Some(if current == "Z" {
                    Arrival::LowEntry(expected_time)
                } else {
                    Arrival::RegularEntry(expected_time)
                });
                break; // early exit after the last needed column
            }
            _ => {}
        }
        start = i.saturating_add(1);
    }

    Ok(StopArrival {
        number: route_number
            .ok_or(ParsingUpstreamError::Error(String::from(
                "invalid arrival data1",
            )))?
            .to_string(),
        r#type: route_type
            .ok_or(ParsingUpstreamError::Error(String::from(
                "invalid arrival data2",
            )))?
            .to_string(),
        arrivals: arrival_type.ok_or(ParsingUpstreamError::Error(String::from(
            "invalid arrival data3",
        )))?,
    })
}

pub fn extract_arrival_list_data(
    arrival_lines: &[u8],
) -> impl Iterator<Item = core::result::Result<StopArrival, ParsingUpstreamError>> {
    let mut start = 0usize;
    memchr_iter(b'\n', arrival_lines)
        .chain(std::iter::once(arrival_lines.len()))
        .map(move |i| {
            let part = &arrival_lines[start..i];
            start = i + 1;
            part
        })
        .filter(|line| !line.is_empty())
        .map(extract_arrival_data)
}

pub fn extract_stop_arrival_list_data(
    stop_lines: &[u8],
    stop_map: &HashMap<String, Rc<StopData>>,
) -> core::result::Result<StopArrivals, ParsingUpstreamError> {
    let first_new_line_pos = memchr::memchr(b'\n', stop_lines).ok_or(
        ParsingUpstreamError::Error(String::from("invalid arrival data4")),
    )?;
    let stop_id = {
        let first_line = remove_trailing_newline(&stop_lines[..=first_new_line_pos]);
        let stop_id_comma_pos =
            memchr::memchr_iter(b',', first_line)
                .next()
                .ok_or(ParsingUpstreamError::Error(String::from(
                    "invalid arrival data5",
                )))?;
        &first_line[stop_id_comma_pos + 1..]
    };

    let mut arrivals = HashMap::new();
    let arrival_lines = &stop_lines[first_new_line_pos + 1..];

    for arrival in extract_arrival_list_data(arrival_lines) {
        let arrival = arrival?;
        arrivals
            .entry(arrival.r#type.clone())
            .or_insert_with(HashMap::new)
            .entry(arrival.number.clone())
            .or_insert_with(Vec::new)
            .push(arrival.arrivals);
    }

    let stop_id = unsafe { str::from_utf8_unchecked(stop_id) };
    Ok(StopArrivals {
        id: stop_id.to_string(),
        name: TransportService::get_stop_name_by_id(stop_id, stop_map)
            .map(|name| name.to_string())
            .ok_or(ParsingUpstreamError::Error(String::from(
                "invalid arrival data6",
            )))?,
        arrivals,
    })
}

pub fn extract_arrival_stop_data_from_line(
    line: &[u8],
    stop_map: &HashMap<String, Rc<StopData>>,
) -> impl Iterator<Item = core::result::Result<StopArrivals, ParsingUpstreamError>> {
    let mut start = 0usize;
    memmem::find_iter(line, b"\nstop,")
        .chain(std::iter::once(line.len()))
        .map(move |i| {
            let part = &line[start..i];
            start = i + 1;
            part
        })
        .filter(|s| memchr::memchr(b'\n', s).is_some())
        .map(|s| extract_stop_arrival_list_data(s, stop_map))
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

#[allow(clippy::type_complexity)]
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
                        r#type: route_data.route_type,
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
        .or_else(|| last_name.as_ref().map(Rc::clone))?;
    let siri_id = siri_id
        .map(str::trim)
        .map(str::to_string)
        .filter(|s| !s.is_empty())?;
    let id = id
        .map(str::trim)
        .map(str::to_string)
        .filter(|s| !s.is_empty())?;

    Some(Rc::new(StopData { id, siri_id, name }))
}

#[allow(clippy::type_complexity)]
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
