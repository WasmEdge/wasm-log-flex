use std::{collections::BTreeMap, sync::Arc, time::Duration};

use tokio::time::sleep;
use wlf_core::{ComponentApi, ComponentKind, Event, EventHub, EventHubApi, EventMeta, Value};

pub struct BinlogCollector {
    id: String,
    destination: String,
}

impl ComponentApi for BinlogCollector {
    fn id(&self) -> &str {
        self.id.as_str()
    }
    fn kind(&self) -> ComponentKind {
        ComponentKind::Collector
    }
}

impl BinlogCollector {
    pub fn new(id: impl Into<String>, destination: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            destination: destination.into(),
        }
    }

    pub async fn start_collecting(self, hub: Arc<EventHub>) {
        loop {
            sleep(Duration::from_secs(1)).await;
            let type_value = Value::String("QueryEvent".to_owned());
            let sql_value = Value::String("HELLO".to_owned());
            hub.send_event(
                Event {
                    value: Value::Object(BTreeMap::from([
                        ("type".to_string(), type_value),
                        ("sql".to_string(), sql_value),
                    ])),
                    meta: EventMeta {},
                },
                self.destination.as_str(),
            )
            .await
            .expect("can't send binlog event");
        }
    }
}
