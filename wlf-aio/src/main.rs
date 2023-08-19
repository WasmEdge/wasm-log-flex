use std::{collections::HashMap, error::Error, fs::File, io::BufReader, path::PathBuf, sync::Arc};

use clap::{command, Parser};
use config::Dispatcher;
use tracing::{error, warn};
use wlf_binlog_collector::BinlogCollector;
use wlf_binlog_filter::{BinlogFilter, BinlogFilterRules};
use wlf_core::event_router::{EventRouter, EventRouterApi};
use wlf_kafka_dispatcher::{CompressionType, KafkaDispatcher};
use wlf_redis_dispatcher::RedisDispatcher;

use crate::config::{Collector, Config, Transformer};

mod config;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    config: PathBuf,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let config_type = cli
        .config
        .extension()
        .expect("can't recognize config file format");
    let config: Config = match config_type.to_str().unwrap() {
        "yaml" => serde_yaml::from_reader(BufReader::new(File::open(cli.config)?))?,
        "properties" => {
            let properties = java_properties::read(BufReader::new(File::open(cli.config)?))?;
            convert_maxwell_java_properties_to_config(properties)
        }
        _ => {
            panic!("can't recognize config file format")
        }
    };

    let mut router = EventRouter::new();
    for c in &config.collectors {
        router.register_component(c.as_component());
    }
    for t in &config.transformers {
        router.register_component(t.as_component());
    }
    for d in &config.dispatchers {
        router.register_component(d.as_component());
    }
    let router = Arc::new(router);

    let mut handles = vec![];
    for c in config.collectors {
        let router_c = Arc::clone(&router);
        let handle = tokio::spawn(async move {
            if let Err(e) = c.as_component().run(router_c).await {
                error!(e);
            }
        });
        handles.push(handle);
    }
    for t in config.transformers {
        let router_c = Arc::clone(&router);
        let handle = tokio::spawn(async move {
            if let Err(e) = t.as_component().run(router_c).await {
                error!(e);
            }
        });
        handles.push(handle);
    }
    for d in config.dispatchers {
        let router_c = Arc::clone(&router);
        let handle = tokio::spawn(async move {
            if let Err(e) = d.as_component().run(router_c).await {
                error!(e);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("something went wrong");
    }

    Ok(())
}

fn convert_maxwell_java_properties_to_config(mut properties: HashMap<String, String>) -> Config {
    let mut config = Config::default();

    fn maxwell_key_to_wlf_format(maxwell: String) -> String {
        maxwell
            .replace("%{table}", "%{/table}")
            .replace("%{database}", "%{/database}")
    }

    // mysql
    let mut collector = BinlogCollector {
        id: "collector".to_string(),
        destination: "filter".to_string(),
        host: properties
            .remove("host")
            .unwrap_or_else(wlf_binlog_collector::default_host),
        user: properties.remove("user").expect("mysql user not specified"),
        password: properties.remove("password").unwrap_or_default(),
        port: properties
            .remove("port")
            .map(|port| port.parse().expect("unknown port number"))
            .unwrap_or_else(wlf_binlog_collector::default_port),
    };

    // filter
    if let Some(rules) = properties.remove("filter") {
        let mut filter = BinlogFilter {
            id: "filter".to_string(),
            destination: "dispatcher".to_string(),
            rules: BinlogFilterRules::new(),
        };
        for rule in rules.split(',') {
            if let Some(dbtb) = rule.strip_prefix("exclude:") {
                let dbtb: Vec<&str> = dbtb.trim().split('.').collect();
                let db = dbtb.first().expect("no database in filter");
                let tb = dbtb.get(1).expect("no table in filter");
                filter.rules.exclude(*db, *tb);
            } else if let Some(dbtb) = rule.strip_prefix("include:") {
                let dbtb: Vec<&str> = dbtb.trim().split('.').collect();
                let db = dbtb.first().expect("no database in filter");
                let tb = dbtb.get(1).expect("no table in filter");
                filter.rules.include(*db, *tb);
            } else {
                panic!("filter broken");
            }
        }
        config.transformers.push(Transformer::BinlogFilter(filter));
    } else {
        collector.destination = "dispatcher".to_string();
    }

    // producer
    let dispatcher_type = properties.remove("producer").expect("no producer");
    let dispatcher = match dispatcher_type.as_str() {
        "kafka" => {
            let dispatcher = KafkaDispatcher {
                id: "dispatcher".to_string(),
                topic: maxwell_key_to_wlf_format(
                    properties
                        .remove("kafka_topic")
                        .unwrap_or_else(wlf_kafka_dispatcher::default_topic),
                ),
                bootstrap_brokers: properties
                    .remove("kafka.bootstrap.servers")
                    .expect("no bootstrap_brokers specified")
                    .split(',')
                    .map(|s| s.to_string())
                    .collect(),
                compression_type: properties
                    .remove("kafka.compression.type")
                    .map(|c| match c.as_str() {
                        "snappy" => CompressionType::Snappy,
                        "gzip" => CompressionType::Gzip,
                        _ => panic!("compression_type not supported"),
                    })
                    .unwrap_or_default(),
            };
            Dispatcher::Kafka(dispatcher)
        }
        "redis" => {
            let redis_key = maxwell_key_to_wlf_format(
                properties
                    .remove("redis_key")
                    .unwrap_or_else(wlf_redis_dispatcher::default_key),
            );
            let redis_stream_json_key = properties
                .remove("redis_stream_json_key")
                .unwrap_or_else(wlf_redis_dispatcher::default_redis_stream_json_key);
            let redis_mode = match properties.remove("redis_type").as_deref() {
                Some("pubsub") => wlf_redis_dispatcher::Mode::Pub { channel: redis_key },
                Some("xadd") => wlf_redis_dispatcher::Mode::XADD {
                    key: redis_stream_json_key,
                },
                Some("lpush") => wlf_redis_dispatcher::Mode::LPush { key: redis_key },
                Some("rpush") => wlf_redis_dispatcher::Mode::RPush { key: redis_key },
                None => wlf_redis_dispatcher::Mode::default(),
                _ => {
                    panic!("redis_type not supported");
                }
            };
            let redis_config = wlf_redis_dispatcher::Config {
                host: properties
                    .remove("redis_host")
                    .unwrap_or_else(wlf_redis_dispatcher::default_host),
                port: properties
                    .remove("redis_port")
                    .map(|port| port.parse().expect("wrong redis_port"))
                    .unwrap_or_else(wlf_redis_dispatcher::default_port),
                auth: properties.remove("redis_auth"),
                database_number: properties
                    .remove("redis_database")
                    .map(|dbn| dbn.parse().expect("wrong redis_database"))
                    .unwrap_or_else(wlf_redis_dispatcher::default_database_number),
            };
            let dispatcher = RedisDispatcher {
                id: "dispatcher".to_string(),
                mode: redis_mode,
                config: redis_config,
            };
            Dispatcher::Redis(dispatcher)
        }
        _ => {
            panic!("unsupported producer type");
        }
    };

    config.collectors.push(Collector::Binlog(collector));
    config.dispatchers.push(dispatcher);

    for (k, v) in properties {
        warn!("unrecognized property: {k}={v}");
    }

    config
}
