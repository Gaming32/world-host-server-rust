use crate::connection::Connection;
use crate::connection::connection_id::ConnectionId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub type SafeConnectionList = Arc<Mutex<Vec<Connection>>>;

pub struct ConnectionSet {
    connections: HashMap<ConnectionId, Connection>,
    connections_by_user_id: HashMap<Uuid, SafeConnectionList>,
}

impl ConnectionSet {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            connections_by_user_id: HashMap::new(),
        }
    }

    pub fn by_id(&self, id: ConnectionId) -> Option<&Connection> {
        self.connections.get(&id)
    }

    pub fn by_user_id(&self, user_id: Uuid) -> Vec<Connection> {
        match self.connections_by_user_id.get(&user_id) {
            Some(connections) => connections.clone().lock().unwrap().clone(),
            None => Vec::default(),
        }
    }

    pub fn add(&mut self, connection: Connection) -> bool {
        if self.connections.contains_key(&connection.id) {
            return false;
        }
        self.add_force(connection)
    }

    pub fn add_force(&mut self, connection: Connection) -> bool {
        let old = self.connections.insert(connection.id, connection.clone());
        let by_uuid_arc = self
            .connections_by_user_id
            .entry(connection.user_uuid)
            .or_insert_with(|| Arc::new(Mutex::new(Vec::new())))
            .clone();
        let mut by_uuid = by_uuid_arc.lock().unwrap();
        if let Some(old) = old
            && let Some(old_pos) = by_uuid.iter().position(|x| x.id == old.id)
        {
            by_uuid.swap_remove(old_pos);
        }
        by_uuid.push(connection);
        true
    }

    pub fn remove(&mut self, connection: &Connection) {
        self.connections.remove(&connection.id);
        let remove =
            if let Some(by_uuid_arc) = self.connections_by_user_id.get(&connection.user_uuid) {
                let mut by_uuid = by_uuid_arc.lock().unwrap();
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

    pub fn iter(&self) -> impl Iterator<Item = &Connection> {
        self.connections.values()
    }
}
