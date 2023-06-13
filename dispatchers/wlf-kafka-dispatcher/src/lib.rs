use std::sync::Arc;

use wlf_core::{ComponentApi, ComponentKind, EventHub, EventHubApi, Value};

pub struct KafkaDispatcher;

impl ComponentApi for KafkaDispatcher {
    fn id(&self) -> &str {
        "KafkaMysqlTableDispatcher"
    }

    fn kind(&self) -> ComponentKind {
        ComponentKind::Dispatcher
    }
}

impl KafkaDispatcher {
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

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use tokio::time::sleep;

    use super::*;
    use wlf_binlog_collector::BinlogCollector;
    use wlf_binlog_parser::BinlogParser;

    #[tokio::test]
    async fn start_log_flex() {
        // create an event hub
        let mut hub = EventHub::new();

        // create a collector, a transformer, and a dispatcher first
        // register them in the `EventHub`
        let collector = BinlogCollector;
        let transformer = BinlogParser;
        let dispatcher = KafkaDispatcher;
        hub.register_component(&collector);
        hub.register_component(&transformer);
        hub.register_component(&dispatcher);

        // start all the components
        let hub = Arc::new(hub);
        let hub_arc = Arc::clone(&hub);
        tokio::spawn(async move {
            collector.start_collecting(hub_arc).await;
        });
        let hub_arc = Arc::clone(&hub);
        tokio::spawn(async move {
            transformer.start_parsing(hub_arc).await;
        });
        let hub_arc = Arc::clone(&hub);
        tokio::spawn(async move {
            dispatcher.start_dispatching(hub_arc).await;
        });

        // should output mysql table event is forwarded to ...

        sleep(Duration::from_secs(10)).await;
    }
}
