pub mod collectors;
pub mod dispatchers;
pub mod transformers;

pub mod event;
pub mod event_hub;

#[derive(Debug, PartialEq, Eq)]
pub enum ComponentKind {
    Collector,
    Transformer,
    Dispatcher,
}

pub trait ComponentApi: 'static + Send + Sync {
    fn id(&self) -> &str;
    fn kind(&self) -> ComponentKind;
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use tokio::time::sleep;

    use crate::{
        collectors::binlog_collector::BinlogCollector,
        dispatchers::kafka_mysql_table_dispatcher::KafkaMysqlTableDispatcher,
        event_hub::{EventHub, EventHubApi},
        transformers::binlog_parser::BinlogTableParser,
    };

    #[tokio::test]
    async fn start_log_flex() {
        // create an event hub
        let mut hub = EventHub::new();

        // create a collector, a transformer, and a dispatcher first
        // register them in the `EventHub`
        let collector = BinlogCollector;
        let transformer = BinlogTableParser;
        let dispatcher = KafkaMysqlTableDispatcher;
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
