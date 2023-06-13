use std::{collections::BTreeMap, sync::Arc, time::Duration};

use tokio::time::sleep;

use crate::{
    event::{Event, EventMeta, Value},
    event_hub::{EventHub, EventHubApi},
    ComponentApi, ComponentKind,
};

pub struct BinlogCollector;
impl ComponentApi for BinlogCollector {
    fn id(&self) -> &str {
        "BinlogCollector"
    }
    fn kind(&self) -> ComponentKind {
        ComponentKind::Collector
    }
}
impl BinlogCollector {
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
                "BinlogTableParser",
            )
            .await
            .expect("can't send binlog event");
        }
    }
}
