use std::{sync::Arc, time::Duration};

use tokio::time::sleep;

use wlf_binlog_collector::BinlogCollector;
use wlf_binlog_parser::BinlogParser;
use wlf_core::{EventHub, EventHubApi};
use wlf_kafka_dispatcher::KafkaDispatcher;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // create an event hub
    let mut hub = EventHub::new();

    // create a collector, a transformer, and a dispatcher first
    // register them in the `EventHub`
    let collector = BinlogCollector::new("binlog_collector", "binlog_parser");
    let transformer = BinlogParser::new("binlog_parser", "kafka_dispatcher");
    let dispatcher = KafkaDispatcher::new("kafka_dispatcher");
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
