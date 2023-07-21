use std::sync::Arc;

use async_trait::async_trait;
use chrono::{LocalResult, TimeZone, Utc};
use futures_util::{pin_mut, StreamExt};
use mysql_cdc::{
    binlog_client::BinlogClient,
    events::{binlog_event::BinlogEvent, event_header::EventHeader},
};

use serde::Deserialize;
use sql_analyzer::SqlAnalyzer;
use tracing::{info, warn};
use wlf_core::{
    event_router::{EventRouter, EventRouterApi},
    value, ComponentApi, ComponentKind, Event, EventMeta, Value,
};

mod error;
mod sql_analyzer;

pub use error::Error;
pub use mysql_cdc::binlog_options::BinlogOptions;
pub use mysql_cdc::replica_options::ReplicaOptions;
pub use mysql_cdc::ssl_mode::SslMode;

#[derive(Deserialize, Debug)]
pub struct BinlogCollector {
    pub id: String,
    pub destination: String,
    #[serde(default = "default_host")]
    pub host: String,
    pub user: String,
    pub password: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

pub fn default_host() -> String {
    "localhost".to_string()
}

pub const fn default_port() -> u16 {
    3306
}

#[async_trait]
impl ComponentApi for BinlogCollector {
    fn id(&self) -> &str {
        self.id.as_str()
    }
    fn kind(&self) -> ComponentKind {
        ComponentKind::Collector
    }

    async fn run(&self, router: Arc<EventRouter>) -> Result<(), Box<dyn std::error::Error>> {
        // create the binlog client
        let mut client = BinlogClient::new(ReplicaOptions {
            username: self.user.clone(),
            password: self.password.clone(),
            ssl_mode: SslMode::Disabled,
            binlog: BinlogOptions::from_end(),
            ..Default::default()
        });

        let events_stream = client.replicate().await?;
        pin_mut!(events_stream);

        // create sql parser
        let mut sql_parser = SqlAnalyzer::new();

        while let Some(Ok((event_header, binlog_event))) = events_stream.next().await {
            info!("new binlog event:\n\t{event_header:?}\n\t{binlog_event:?}");
            match into_wlf_event(&mut sql_parser, event_header, binlog_event) {
                Ok(Some(event)) => router.send_event(event, &self.destination).await?,
                Ok(None) => {}
                Err(e) => warn!("failed to convert binlog event, {e}"),
            }
        }

        Ok(())
    }
}

/// The event structure is largely borrowed from [maxwells](https://maxwells-daemon.io/dataformat/)
fn into_wlf_event(
    sql_analyzer: &mut SqlAnalyzer,
    event_header: EventHeader,
    binlog_event: BinlogEvent,
) -> Result<Option<Event>, Error> {
    let LocalResult::Single(timestamp) = Utc.timestamp_opt(event_header.timestamp as i64, 0) else {
        return Err(Error::Other("failed to convert timestamp".to_string()));
    };
    match binlog_event {
        BinlogEvent::QueryEvent(e) => {
            let mut value = value!({
                "timestamp": timestamp,
                "server_id": event_header.server_id,
                "thread_id": e.thread_id,
            });

            let mut sql_properties = sql_analyzer.analyze(&e.database_name, &e.sql_statement)?;
            if sql_properties.is_null() {
                return Ok(None);
            }

            value
                .as_object_mut()
                .unwrap()
                .append(sql_properties.as_object_mut().unwrap());

            Ok(Some(Event {
                value,
                meta: EventMeta {},
            }))
        }
        BinlogEvent::TableMapEvent(e) => {
            sql_analyzer.map_table(&e.database_name, &e.table_name, e.table_id);
            Ok(None)
        }
        BinlogEvent::WriteRowsEvent(e) => {
            let (database, table) = sql_analyzer.get_table_info(e.table_id)?;
            let columns = sql_analyzer.get_column_defs(e.table_id)?;
            let mut value = value!({
                "database": database,
                "table": table,
                "type": "insert",
                "timestamp": timestamp,
                "server_id": event_header.server_id,
                "data": []
            });
            let data = value.pointer_mut("/data").unwrap().as_array_mut().unwrap();
            for (i, r) in e.rows.iter().enumerate() {
                let mut row_value = value!({});
                let row_data = row_value.as_object_mut().unwrap();
                for c in &r.cells {
                    let Some(def) = columns.get(i) else {
                        warn!("row data and column definitions do not match");
                        break;
                    };
                    row_data.insert(
                        def.name.to_string(),
                        c.as_ref().map_or(Value::Null, |v| format!("{v:?}").into()),
                    );
                }
                data.push(row_value);
            }
            Ok(Some(Event {
                value,
                meta: EventMeta {},
            }))
        }
        BinlogEvent::RotateEvent(_)
        | BinlogEvent::UnknownEvent
        | BinlogEvent::FormatDescriptionEvent(_)
        | BinlogEvent::HeartbeatEvent(_)
        | BinlogEvent::XidEvent(_) => Ok(None),
        _ => Err(Error::Other("unsupported binlog event".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use utils::test_utils::DummyComponent;
    use wlf_core::{
        event_router::{EventRouter, EventRouterApi},
        ComponentApi, ComponentKind,
    };

    use crate::{default_host, default_port, BinlogCollector};

    #[tokio::test]
    async fn collect() {
        let collector = BinlogCollector {
            id: "binlog_collector".to_string(),
            user: "root".to_string(),
            destination: "dispatcher".to_string(),
            host: default_host(),
            password: "password".to_string(),
            port: default_port(),
        };

        let dummy_dispatcher = DummyComponent::new("dispatcher", ComponentKind::Dispatcher);

        let mut router = EventRouter::new();
        router.register_component(&collector);
        router.register_component(&dummy_dispatcher);
        let router = Arc::new(router);

        let router_c = Arc::clone(&router);
        tokio::spawn(async move {
            collector
                .run(Arc::clone(&router_c))
                .await
                .expect("failed to run collector");
        });

        while let Ok(event) = router.poll_event("dispatcher").await {
            println!("{event:#?}");
        }
    }
}
