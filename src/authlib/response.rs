use serde::{Deserialize, Serialize};
use uuid::Uuid;
use uuid::serde::simple;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HasJoinedMinecraftServerResponse {
    #[serde(with = "simple")]
    pub id: Uuid,
}
