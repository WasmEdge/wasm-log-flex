use std::sync::Arc;

use crate::{
    event_hub::{EventHub, EventHubApi},
    ComponentApi, ComponentKind, Event,
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
            let event = if let Event::BinlogParsed(e) = event {
                e
            } else {
                eprintln!("receive wrong event type");
                continue;
            };

            // dispatch event to corresponding kafka table
            println!(
                "{:?} is dispatched to {} topic",
                event.binlog_event, event.table
            );
        }
    }
}
