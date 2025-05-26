use crate::connection::Connection;
use crate::connection::connection_id::ConnectionId;
use dashmap::DashMap;
use dashmap::mapref::multiple::RefMulti;
use dashmap::mapref::one::Ref;
use uuid::Uuid;

pub struct ConnectionSet {
    connections: DashMap<ConnectionId, Connection>,
    connections_by_user_id: DashMap<Uuid, Vec<Connection>>,
}

impl ConnectionSet {
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
            connections_by_user_id: DashMap::new(),
        }
    }

    pub fn by_id(&self, id: ConnectionId) -> Option<Ref<ConnectionId, Connection>> {
        self.connections.get(&id)
    }

    pub fn by_user_id(&self, user_id: Uuid) -> Vec<Connection> {
        match self.connections_by_user_id.get(&user_id) {
            Some(connections) => connections.value().clone(),
            None => Vec::default(),
        }
    }

    pub fn add(&self, connection: Connection) -> bool {
        if self.connections.contains_key(&connection.id) {
            return false;
        }
        self.add_force(connection)
    }

    pub fn add_force(&self, connection: Connection) -> bool {
        let old = self.connections.insert(connection.id, connection.clone());
        let mut by_uuid = self
            .connections_by_user_id
            .entry(connection.user_uuid)
            .or_default();
        if let Some(old) = old {
            if let Some(old_pos) = by_uuid.iter().position(|x| x.id == old.id) {
                by_uuid.swap_remove(old_pos);
            }
        }
        by_uuid.push(connection);
        true
    }

    pub fn remove(&self, connection: &Connection) {
        self.connections.remove(&connection.id);
        let remove =
            if let Some(mut by_uuid) = self.connections_by_user_id.get_mut(&connection.user_uuid) {
                if let Some(old_pos) = by_uuid.iter().position(|x| x.id == connection.id) {
                    by_uuid.swap_remove(old_pos);
                }
                by_uuid.is_empty()
            } else {
                false
            };
        if remove {
            self.connections_by_user_id.remove(&connection.user_uuid);
        }
    }

    pub fn len(&self) -> usize {
        self.connections.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = RefMulti<ConnectionId, Connection>> {
        self.connections.iter()
    }
}
