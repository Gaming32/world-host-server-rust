use crate::lat_long::LatitudeLongitude;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ExternalProxy {
    pub lat_long: LatitudeLongitude,

    pub addr: Option<String>,

    #[serde(default = "default_port")]
    pub port: u16,

    pub base_addr: Option<String>,

    #[serde(default = "default_mc_port")]
    pub mc_port: u16,
}

fn default_port() -> u16 {
    9656
}

fn default_mc_port() -> u16 {
    25565
}
