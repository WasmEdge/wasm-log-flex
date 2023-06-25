use std::sync::Arc;

use chrono::{LocalResult, TimeZone, Utc};
use futures_util::{pin_mut, StreamExt};
use mysql_cdc::{
    binlog_client::BinlogClient,
    events::{binlog_event::BinlogEvent, event_header::EventHeader},
};

use sql_analyzer::SqlAnalyzer;
use tracing::{error, info};
use wlf_core::{
    event_router::{EventRouter, EventRouterApi},
    ComponentApi, ComponentKind, Event, EventMeta, Value,
};

mod error;
mod sql_analyzer;

pub use error::Error;
pub use mysql_cdc::binlog_options::BinlogOptions;
pub use mysql_cdc::replica_options::ReplicaOptions;
pub use mysql_cdc::ssl_mode::SslMode;

pub struct BinlogCollector {
    id: String,
    destination: String,
    replica_options: ReplicaOptions,
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
    pub fn new(
        id: impl Into<String>,
        destination: impl Into<String>,
        replica_options: ReplicaOptions,
    ) -> Self {
        Self {
            id: id.into(),
            destination: destination.into(),
            replica_options,
        }
    }

    pub async fn start_collecting(self, router: Arc<EventRouter>) -> Result<(), Error> {
        // create the binlog client
        let mut client = BinlogClient::new(self.replica_options);
        let events_stream = client.replicate().await?;
        pin_mut!(events_stream);

        // create sql parser
        let mut sql_parser = SqlAnalyzer::new();

        while let Some(Ok((event_header, binlog_event))) = events_stream.next().await {
            info!("new binlog event:\n{event_header:#?}\n{binlog_event:#?}");
            match into_wlf_event(&mut sql_parser, event_header, binlog_event) {
                Ok(event) => router.send_event(event, &self.destination).await?,
                Err(e) => error!("failed to convert binlog event, {e}"),
            }
        }

        Ok(())
    }
}

/// The event structure is largely borrowed from [maxwells](https://maxwells-daemon.io/dataformat/)
fn into_wlf_event(
    sql_parser: &mut SqlAnalyzer,
    event_header: EventHeader,
    binlog_event: BinlogEvent,
) -> Result<Event, Error> {
    match binlog_event {
        BinlogEvent::QueryEvent(e) => {
            info!("receive query event {e:?}");

            let LocalResult::Single(timestamp) = Utc.timestamp_opt(event_header.timestamp as i64, 0) else {
                return Err(Error::Other("failed to convert timestamp".to_string()));
            };
            let meta = Value::from([
                ("database", e.database_name.into()),
                ("timestamp", timestamp.into()),
                ("server_id", event_header.server_id.into()),
                ("thread_id", e.thread_id.into()),
            ]);
            let properties = sql_parser.analyze(&e.sql_statement)?;

            let value = Value::from([("meta", meta), ("sql", properties)]);

            Ok(Event {
                value,
                meta: EventMeta {},
            })
        }
        _ => Err(Error::Other("unsupported binlog event".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use mysql_cdc::{
        binlog_options::BinlogOptions, replica_options::ReplicaOptions, ssl_mode::SslMode,
    };
    use utils::test_utils::DummyComponent;
    use wlf_core::{
        event_router::{EventRouter, EventRouterApi},
        ComponentKind,
    };

    use crate::BinlogCollector;

    #[tokio::test]
    async fn collect() {
        let options = ReplicaOptions {
            username: String::from("root"),
            password: String::from("password"),
            blocking: true,
            ssl_mode: SslMode::Disabled,
            binlog: BinlogOptions::from_start(),
            ..Default::default()
        };
        let collector = BinlogCollector::new("binlog_collector", "test", options);

        let dummy_dispatcher = DummyComponent::new("dispatcher", ComponentKind::Dispatcher);

        let mut router = EventRouter::new();
        router.register_component(&collector);
        router.register_component(&dummy_dispatcher);
        let router = Arc::new(router);

        tokio::spawn(collector.start_collecting(Arc::clone(&router)));

        while let Ok(event) = router.poll_event("dispatcher").await {
            println!("{event:#?}");
        }
    }
}
