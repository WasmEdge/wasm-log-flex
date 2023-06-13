use std::sync::Arc;

use crate::{
    event::Value,
    event_hub::{EventHub, EventHubApi},
    ComponentApi, ComponentKind,
};

pub struct BinlogTableParser;
impl ComponentApi for BinlogTableParser {
    fn id(&self) -> &str {
        "BinlogTableParser"
    }
    fn kind(&self) -> ComponentKind {
        ComponentKind::Transformer
    }
}
impl BinlogTableParser {
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

            hub.send_event(event, "KafkaMysqlTableDispatcher")
                .await
                .expect("can't send out event");
        }
    }
}
