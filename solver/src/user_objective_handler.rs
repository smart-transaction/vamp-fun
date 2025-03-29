use crate::appchain_listener::Handler;

mod proto {
    tonic::include_proto!("vamp.fun");
}

use proto::UserObjective;

pub struct UserObjectiveHandler {}

impl UserObjectiveHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl Handler<UserObjective> for UserObjectiveHandler {
    async fn handle(&mut self, event: UserObjective) {
        println!("Received UserObjective: {:?}", event);
    }
}
