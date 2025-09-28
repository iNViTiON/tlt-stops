use serde::{self, Deserialize, Serialize};

pub struct RouteData {
    pub number: String,
    pub route_type: String,
    pub directions: String,
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
    pub directions: Vec<String>,
}
