use serde::{self, Deserialize, Serialize};
use std::collections::HashMap;
use std::rc::Rc;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteGroup {
    pub number: String,
    #[serde(rename = "type")]
    pub route_type: String,
    pub directions: HashMap<String, Vec<String>>,
}

pub struct StopData {
    pub id: String,
    pub siri_id: String,
    pub name: Rc<String>,
}
