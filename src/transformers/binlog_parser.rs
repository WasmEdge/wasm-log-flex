use std::sync::Arc;

use crate::{
    event_hub::{EventHub, EventHubApi},
    events::BinlogParsedEvent,
    ComponentApi, ComponentKind, Event,
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
        while let Ok(event) = hub.poll_event(self.id()).await {
            let event = if let Event::Binlog(e) = event {
                e
            } else {
                eprintln!("receive wrong event type");
                continue;
            };

            // parse sql and get table info...
            let table = event.sql.clone();

            let new_event = BinlogParsedEvent {
                binlog_event: event,
                table,
            };
            hub.send_event(Event::BinlogParsed(new_event), "KafkaMysqlTableDispatcher")
                .await
                .expect("can't send out event");
        }
    }
}
