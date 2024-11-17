use crate::connection::connection_id::ConnectionId;
use crate::country_code::CountryCode;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub mod connection_id;
pub mod connection_set;

#[derive(Clone)]
pub struct Connection {
    pub id: ConnectionId,
    pub user_uuid: Uuid,
    pub live: Arc<Mutex<LiveConnection>>,
}

pub struct LiveConnection {
    pub country: Option<CountryCode>,
}
