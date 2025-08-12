use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum DataCell {
    Double(f64),
    BigInt(i64),
    Text(String),
    Null,
}

impl Default for DataCell {
    fn default() -> Self { return Self::Null }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CysQuery {
    pub cys_name: String,
    pub screen_name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq)]
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
            "double precision" => Some(ColType::DOUBLE),
            "text" => Some(ColType::TEXT),
            _ => None,
        }
    }
}
