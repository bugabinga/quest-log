use crate::database::Database;
use crate::handlers::ServerMessage;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub bcast: broadcast::Sender<ServerMessage>,
}

impl AppState {
    pub fn new(db: Database, bcast: broadcast::Sender<ServerMessage>) -> Self {
        Self { db, bcast }
    }
}
