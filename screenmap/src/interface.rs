use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub type ScreenmapRow = BTreeMap<String, String>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CysQuery {
    pub cys_name: String,
    pub screen_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WSQuery {
    SetScreen(String),
    QueryRow((usize, String)),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WSResponse {
    Confirm(String),
    RespondRow((ScreenmapRow, usize)),
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum ColType {
    SMALLINT = 0,
    INT = 1,
    BIGINT = 2,
    REAL = 3,
    DOUBLE = 4,
    TEXT = 5,
}

impl ColType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "smallint" => Some(ColType::SMALLINT),
            "integer" => Some(ColType::INT),
            "bigint" => Some(ColType::BIGINT),
            "real" => Some(ColType::REAL),
            "double" => Some(ColType::DOUBLE),
            "text" => Some(ColType::TEXT),
            _ => None,
        }
    }
}
