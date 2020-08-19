use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ResponseData {
    pub start_time: String,
    pub end_time: String,
    pub items: Vec<DataEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DataEntry {
    pub start_time: String,
    pub end_time: String,
    pub percentage: u8,
    pub level: Level,
    pub is_current: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Level {
    LOW,
    NORMAL,
    HIGH,
}
