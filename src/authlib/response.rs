use serde::{Deserialize, Serialize};
use uuid::serde::simple;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HasJoinedMinecraftServerResponse {
    #[serde(with = "simple")]
    pub id: Uuid,
}
