use crate::appchain_listener::Handler;
use crate::state_snapshot::StateSnapshot;
use crate::use_proto::proto::{StateSnapshotProto, UserObjectiveProto};
use log::info;

pub struct StateSnapshotHandler {
    state_snapshot: StateSnapshot,
}

impl StateSnapshotHandler {
    pub fn new() -> Self {
        Self {
            state_snapshot: StateSnapshot::new(),
        }
    }
}

impl Handler<StateSnapshotProto> for StateSnapshotHandler {
    async fn handle(&mut self, event: StateSnapshotProto) {
        info!("Received StateSnapshot: {:?}", event);
        self.state_snapshot = StateSnapshot::from_event(event);
    }
}

pub struct UserObjectiveHandler {}

impl UserObjectiveHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl Handler<UserObjectiveProto> for UserObjectiveHandler {
    async fn handle(&mut self, event: UserObjectiveProto) {
        info!("Received UserObjective: {:?}", event);
    }
}
