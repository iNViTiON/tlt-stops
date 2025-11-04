use serde::ser::SerializeMap;
use serde::{self, Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;
use utoipa::ToSchema;

use crate::caches::CacheDataWithKeys;

pub enum RequestError {
    MissingParameter(String),
    InvalidParameter(String),
}

pub struct RouteData {
    pub number: String,
    pub route_type: String,
    pub directions: String,
    pub stops: Vec<String>,
}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct RouteDirection {
//     pub name: String,
//     pub stops: Vec<String>,
// }

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "number": "1",
    "type": "bus",
    "directions": {
        "Kopli": ["1001", "1002"],
        "Linnahall": ["2001", "2002"]
    }
}))]
pub struct RouteGroup {
    #[schema(example = "1")]
    pub number: String,
    #[schema(example = "bus")]
    pub r#type: String,
    #[schema(example = json!({"Kopli": ["1001", "1002"]}))]
    pub directions: HashMap<String, Vec<String>>,
}

pub struct StopData {
    pub id: String,
    pub siri_id: String,
    pub name: Rc<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!(["1001", "Stop Name"]))]
pub struct StopResponse(pub String, pub String);

// string as ISO8601
#[derive(ToSchema)]
// #[serde(untagged)]
pub enum Arrival {
    RegularEntry(String),
    LowEntry(String),
}

impl Serialize for Arrival {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        match self {
            Arrival::RegularEntry(time) => {
                map.serialize_entry("time", time)?;
            }
            Arrival::LowEntry(time) => {
                map.serialize_entry("time", time)?;
                map.serialize_entry("isLowEntry", &true)?;
            }
        }
        map.end()
    }
}

#[derive(Serialize, ToSchema)]
pub struct StopArrival {
    pub number: String,
    pub r#type: String,
    pub arrivals: Arrival,
}

#[derive(Serialize, ToSchema)]
pub struct StopArrivals {
    pub id: String,
    pub name: String,
    pub arrivals: HashMap<String, HashMap<String, Vec<Arrival>>>,
    // pub arrivals: HashMap<String, HashMap<String, Vec<StopArrival>>>,
}

#[derive(Serialize, ToSchema)]
pub struct PostArrivalsResponse {
    #[schema(value_type = Vec<Option<StopArrivals>>)]
    pub stops: Vec<Option<Rc<StopArrivals>>>,
}

pub struct StopId(pub String);
impl Deref for StopId {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
pub struct ValidStop {
    pub data: Rc<StopData>,
}
pub struct ReadyStopArrivals(pub Rc<StopArrivals>);

pub enum StopArrivalState {
    StopId(StopId),
    Invalid,
    Valid(ValidStop),
    Ready(ReadyStopArrivals),
}

impl StopId {
    pub fn validate(self, stop_map: &HashMap<String, Rc<StopData>>) -> StopArrivalState {
        let id: String = self.to_string();
        if let Some(data) = stop_map.get(&id) {
            StopArrivalState::Valid(ValidStop {
                data: Rc::clone(data),
            })
        } else {
            StopArrivalState::Invalid
        }
    }
}

impl ValidStop {
    pub fn fetch_arrivals_from_cache(
        self,
        arrivals_cache: &CacheDataWithKeys<String, StopArrivals>,
    ) -> StopArrivalState {
        let from_cache = arrivals_cache.get(&self.data.siri_id);
        if let Some(arrivals) = from_cache {
            StopArrivalState::Ready(ReadyStopArrivals(arrivals))
        } else {
            StopArrivalState::Valid(self)
        }
    }
}
