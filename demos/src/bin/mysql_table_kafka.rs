use std::sync::Arc;

use wlf_binlog_collector::{BinlogCollector, BinlogOptions, ReplicaOptions, SslMode};
use wlf_binlog_filter::{BinlogFilter, BinlogFilterRules};
use wlf_core::event_router::{EventRouter, EventRouterApi};
use wlf_kafka_dispatcher::KafkaDispatcher;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt::init();
    // create an event router
    let mut router = EventRouter::new();

    // create a collector, a transformer, and a dispatcher first
    let options = ReplicaOptions {
        username: String::from("root"),
        password: String::from("password"),
        blocking: true,
        ssl_mode: SslMode::Disabled,
        binlog: BinlogOptions::from_end(),
        ..Default::default()
    };
    let collector = BinlogCollector::new("binlog_collector", "binlog_parser", options);

    let rules = BinlogFilterRules::new()
        .exclude("d1", None)
        .include("d1", Some("t1"));
    let transformer = BinlogFilter::new("binlog_parser", "kafka_dispatcher", rules);

    let mut dispatcher =
        KafkaDispatcher::new("kafka_dispatcher", vec!["127.0.0.1:9092".to_string()]);
    dispatcher.set_topic_template(r"logFlex.%{/meta/database}.%{/sql/table}");

    // register them in the router
    router.register_component(&collector);
    router.register_component(&transformer);
    router.register_component(&dispatcher);

    // start all the components
    let router = Arc::new(router);
    let router_arc = Arc::clone(&router);
    tokio::task::spawn(async move {
        collector
            .start_collecting(router_arc)
            .await
            .expect("collector exit unexpectedly");
    });
    let router_arc = Arc::clone(&router);
    tokio::spawn(async move {
        transformer
            .start_filtering(router_arc)
            .await
            .expect("filter exit unexpectedly");
    });
    let router_arc = Arc::clone(&router);
    let handle = tokio::spawn(async move {
        dispatcher
            .start_dispatching(router_arc)
            .await
            .expect("kafka dispatcher exit unexpectedly");
    });

    // should output mysql table event is forwarded to ...

    handle.await.expect("failed");
}
