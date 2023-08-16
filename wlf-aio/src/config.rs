use serde::Deserialize;
use wlf_binlog_collector::BinlogCollector;
use wlf_binlog_filter::BinlogFilter;
use wlf_core::ComponentApi;
use wlf_event_replicator::EventReplicator;
use wlf_kafka_dispatcher::KafkaDispatcher;
use wlf_redis_dispatcher::RedisDispatcher;

#[derive(Deserialize, Debug, Default)]
pub(crate) struct Config {
    #[serde(default)]
    pub(crate) collectors: Vec<Collector>,
    #[serde(default)]
    pub(crate) transformers: Vec<Transformer>,
    #[serde(default)]
    pub(crate) dispatchers: Vec<Dispatcher>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub(crate) enum Collector {
    BinlogCollector(BinlogCollector),
}

impl Collector {
    pub(crate) fn as_component(&self) -> &dyn ComponentApi {
        match self {
            Collector::BinlogCollector(c) => c,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub(crate) enum Transformer {
    BinlogFilter(BinlogFilter),
    EventReplicator(EventReplicator),
}

impl Transformer {
    pub(crate) fn as_component(&self) -> &dyn ComponentApi {
        match self {
            Transformer::BinlogFilter(t) => t,
            Transformer::EventReplicator(t) => t,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub(crate) enum Dispatcher {
    KafkaDispatcher(KafkaDispatcher),
    RedisDispatcher(RedisDispatcher),
}

impl Dispatcher {
    pub(crate) fn as_component(&self) -> &dyn ComponentApi {
        match self {
            Dispatcher::KafkaDispatcher(d) => d,
            Dispatcher::RedisDispatcher(d) => d,
        }
    }
}
