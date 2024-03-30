use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Telemetry{
    pub temperature: f64,
    pub humidity: f64,
    pub pressure: Option<f64>,
    pub device_id: Option<String>,
    pub time_stamp: Option<i64>
}