use std::{sync::Arc, time::Duration};

use tokio::time::sleep;

use crate::{
    event_hub::{EventHub, EventHubApi},
    events::BinlogEvent,
    ComponentApi, ComponentKind, Event,
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
            hub.send_event(
                Event::Binlog(BinlogEvent {
                    sql: "HELLO".to_string(),
                }),
                "BinlogTableParser",
            )
            .await
            .expect("can't send binlog event");
        }
    }
}
