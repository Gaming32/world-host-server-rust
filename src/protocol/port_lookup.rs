use crate::connection::connection_id::ConnectionId;
use std::time::Duration;
use uuid::Uuid;

pub const PORT_LOOKUP_EXPIRY: Duration = Duration::from_secs(10);

#[derive(Copy, Clone, Debug)]
pub struct ActivePortLookup {
    pub lookup_id: Uuid,
    pub source_client: ConnectionId,
}
