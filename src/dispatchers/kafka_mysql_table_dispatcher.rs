use std::sync::Arc;

use crate::{
    event::Value,
    event_hub::{EventHub, EventHubApi},
    ComponentApi, ComponentKind,
};

pub struct KafkaMysqlTableDispatcher;
impl ComponentApi for KafkaMysqlTableDispatcher {
    fn id(&self) -> &str {
        "KafkaMysqlTableDispatcher"
    }
    fn kind(&self) -> ComponentKind {
        ComponentKind::Dispatcher
    }
}
impl KafkaMysqlTableDispatcher {
    pub async fn start_dispatching(self, hub: Arc<EventHub>) {
        while let Ok(event) = hub.poll_event(self.id()).await {
            let Some(Value::String(table)) = event.value.pointer("/table") else {
                eprintln!("no `table` field or `table` is not string, event: {event:?}");
                continue;
            };

            // dispatch event to corresponding kafka table
            println!("{:?} is dispatched to {} topic", event, table);
        }
    }
}
