use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::string::FromUtf8Error;

use chrono::{Timelike, Utc};
use memchr::{memchr_iter, memmem};
use worker::Result;

use crate::models::*;
use crate::services::*;

const DAY_SECS: u32 = 86_400;
/// A departure further in the past than this belongs to the next day.
const PAST_TOLERANCE_SECS: i32 = 3 * 3600;
/// A departure further ahead than this belongs to the previous day.
const FUTURE_TOLERANCE_SECS: i32 = 12 * 3600;

/// Absolute time reference, resolved once per upstream batch.
///
/// Upstream reports a departure as seconds from midnight, but which midnight
/// depends on when you ask: before midnight a 00:10 departure comes as 87000
/// (service day), after midnight the feed rebases to the new calendar day and
/// the same departure comes as 600. Anchoring each line to `now` rather than to
/// a calendar date lands both encodings on the same instant, and costs two
/// integer compares per arrival instead of two timezone conversions.
#[derive(Clone, Copy)]
pub struct DayAnchor {
    now_ms: i64,
    now_local_secs: i32,
}

impl DayAnchor {
    pub fn now() -> Self {
        use chrono_tz::Europe::Tallinn;
        let now = Utc::now();
        Self::new(
            now.timestamp_millis(),
            now.with_timezone(&Tallinn)
                .time()
                .num_seconds_from_midnight() as i32,
        )
    }

    pub fn new(now_ms: i64, now_local_secs: i32) -> Self {
        Self {
            now_ms,
            now_local_secs,
        }
    }

    /// Seconds from midnight (either encoding) to Unix epoch milliseconds.
    ///
    /// Past departures stay in the past — a bus that left 30s ago must keep
    /// reading as 30s ago, not roll a day forward.
    #[inline(always)]
    pub fn to_epoch_ms(self, seconds_from_midnight: u32) -> i64 {
        let mut delta = (seconds_from_midnight % DAY_SECS) as i32 - self.now_local_secs;
        if delta < -PAST_TOLERANCE_SECS {
            delta += DAY_SECS as i32;
        } else if delta > FUTURE_TOLERANCE_SECS {
            delta -= DAY_SECS as i32;
        }
        self.now_ms + delta as i64 * 1000
    }
}

#[cfg(test)]
mod day_anchor_tests {
    use super::DayAnchor;

    /// Arbitrary fixed instant; only deltas matter.
    const NOW_MS: i64 = 1_786_000_000_000;

    fn at(local_hms: (u32, u32, u32)) -> DayAnchor {
        let (h, m, s) = local_hms;
        DayAnchor::new(NOW_MS, (h * 3600 + m * 60 + s) as i32)
    }

    fn delta_secs(anchor: &DayAnchor, seconds_from_midnight: u32) -> i64 {
        (anchor.to_epoch_ms(seconds_from_midnight) - NOW_MS) / 1000
    }

    #[test]
    fn service_day_encoding_before_midnight() {
        // 23:55 now, 00:10 departure sent as 86400 + 600
        assert_eq!(delta_secs(&at((23, 55, 0)), 87_000), 900);
    }

    #[test]
    fn rebased_encoding_before_midnight() {
        // same departure sent as plain 600
        assert_eq!(delta_secs(&at((23, 55, 0)), 600), 900);
    }

    #[test]
    fn service_day_encoding_after_midnight() {
        // 00:05 now, feed still on yesterday's service day
        assert_eq!(delta_secs(&at((0, 5, 0)), 87_000), 300);
    }

    #[test]
    fn rebased_encoding_after_midnight() {
        assert_eq!(delta_secs(&at((0, 5, 0)), 600), 300);
    }

    #[test]
    fn midday_is_untouched() {
        assert_eq!(delta_secs(&at((12, 0, 0)), 43_500), 300);
    }

    #[test]
    fn recent_past_stays_in_the_past() {
        // departed 30s ago — must not roll a day forward, frontend renders "Now"
        assert_eq!(delta_secs(&at((12, 0, 0)), 43_170), -30);
    }

    #[test]
    fn ordering_holds_across_midnight() {
        let anchor = at((23, 55, 0));
        let late = anchor.to_epoch_ms(86_280); // 23:58
        let early = anchor.to_epoch_ms(600); // 00:10 next day
        assert!(late < early);
    }

    #[test]
    fn parsed_feed_orders_across_midnight() {
        use crate::models::{Arrival, StopData};
        use std::collections::HashMap;
        use std::rc::Rc;

        let mut stop_map: HashMap<String, Rc<StopData>> = HashMap::new();
        stop_map.insert(
            "6601".to_string(),
            Rc::new(StopData {
                id: "6601".to_string(),
                siri_id: "6601".to_string(),
                name: Rc::new("Test stop".to_string()),
            }),
        );

        // 23:58 tonight, then 00:10 tomorrow sent rebased as 600.
        let feed = b"Transport,RouteNum,ExpectedTimeInSeconds,ScheduleTimeInSeconds,6601,version20201024\n\
                     stop,6601\n\
                     bus,23,86280,86280,a,b,N\n\
                     bus,23,600,600,a,b,Z\n";

        let anchor = at((23, 55, 0));
        let stops: Vec<_> = super::split_arrival_by_stops(feed)
            .flat_map(|segment| {
                super::extract_arrival_stop_data_from_line(segment, &stop_map, anchor)
            })
            .collect();

        assert_eq!(stops.len(), 1);
        let stop = stops.into_iter().next().unwrap().expect("parse failed");
        assert_eq!(stop.id, "6601");
        assert_eq!(stop.name, "Test stop");

        let times: Vec<i64> = stop.arrivals["bus"]["23"]
            .iter()
            .map(|arrival| match arrival {
                Arrival::RegularEntry(time) | Arrival::LowEntry(time) => *time,
            })
            .collect();
        assert_eq!(times, vec![NOW_MS + 180_000, NOW_MS + 900_000]);
        assert!(matches!(stop.arrivals["bus"]["23"][1], Arrival::LowEntry(_)));
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
    anchor: DayAnchor,
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
                    anchor.to_epoch_ms(
                        current
                            .parse::<u32>()
                            .map_err(|_| ParsingUpstreamError::Utf8)?,
                    ),
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
    anchor: DayAnchor,
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
        .map(move |line| extract_arrival_data(line, anchor))
}

pub fn extract_stop_arrival_list_data(
    stop_lines: &[u8],
    stop_map: &HashMap<String, Rc<StopData>>,
    anchor: DayAnchor,
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

    for arrival in extract_arrival_list_data(arrival_lines, anchor) {
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
    anchor: DayAnchor,
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
        .map(move |s| extract_stop_arrival_list_data(s, stop_map, anchor))
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
