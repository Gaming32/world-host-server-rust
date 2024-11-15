use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use crate::connection::connection_id::ConnectionId;

pub mod connection_set;
pub mod connection_id;

#[derive(Clone)]
pub struct Connection {
    pub id: ConnectionId,
    pub user_uuid: Uuid,
    pub live: Arc<Mutex<LiveConnection>>,
}

pub struct LiveConnection {
}
