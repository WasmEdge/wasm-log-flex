use std::sync::Arc;

use wlf_core::{ComponentApi, ComponentKind, EventHub, EventHubApi, Value};

pub struct KafkaDispatcher {
    id: String,
}

impl ComponentApi for KafkaDispatcher {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn kind(&self) -> ComponentKind {
        ComponentKind::Dispatcher
    }
}

impl KafkaDispatcher {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

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
