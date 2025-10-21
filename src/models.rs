use serde::{self, Deserialize, Serialize};
use std::collections::HashMap;
use std::rc::Rc;
use utoipa::ToSchema;

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
    #[serde(rename = "type")]
    #[schema(example = "bus")]
    pub route_type: String,
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
