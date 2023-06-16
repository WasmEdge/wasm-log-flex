use std::sync::Arc;

use wlf_core::{ComponentApi, ComponentKind, EventHub, EventHubApi, Value};

pub struct BinlogParser {
    id: String,
    destination: String,
}

impl ComponentApi for BinlogParser {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn kind(&self) -> ComponentKind {
        ComponentKind::Transformer
    }
}

impl BinlogParser {
    pub fn new(id: impl Into<String>, destination: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            destination: destination.into(),
        }
    }
    pub async fn start_parsing(self, hub: Arc<EventHub>) {
        while let Ok(mut event) = hub.poll_event(self.id()).await {
            let Some(Value::String(sql)) = event.value.pointer("/sql") else {
                eprintln!("no `sql` field or `sql` is not string, event: {event:?}");
                continue;
            };

            // parse sql and get table info...
            let table = sql.clone();

            let Some(object) = event.value.as_object_mut() else {
                eprintln!("event is not an object, event: {event:?}");
                continue;
            };

            object.insert("table".to_string(), Value::String(table));

            hub.send_event(event, self.destination.as_str())
                .await
                .expect("can't send out event");
        }
    }
}
