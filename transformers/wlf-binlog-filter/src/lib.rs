use std::sync::Arc;

use thiserror::Error;
use tracing::info;
use wlf_core::{
    event_hub::{EventHub, EventHubApi},
    ComponentApi, ComponentKind, Event, Value,
};

pub struct BinlogFilter {
    id: String,
    destination: String,
    rules: BinlogFilterRules,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("event hub error, {0}")]
    EventHub(#[from] wlf_core::event_hub::Error),
}

impl ComponentApi for BinlogFilter {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn kind(&self) -> ComponentKind {
        ComponentKind::Transformer
    }
}

impl BinlogFilter {
    pub fn new(
        id: impl Into<String>,
        destination: impl Into<String>,
        rules: BinlogFilterRules,
    ) -> Self {
        Self {
            id: id.into(),
            destination: destination.into(),
            rules,
        }
    }
    pub async fn start_filtering(self, hub: Arc<EventHub>) -> Result<(), Error> {
        while let Ok(event) = hub.poll_event(self.id()).await {
            info!("{} receives new event:\n{event:#?}", self.id);

            if !self.rules.eval(&event) {
                continue;
            }

            hub.send_event(event, self.destination.as_str()).await?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct BinlogFilterRules {
    rules: Vec<BinlogFilterRule>,
}

enum BinlogFilterRule {
    Include {
        database: String,
        table: Option<String>,
    },
    Exclude {
        database: String,
        table: Option<String>,
    },
}

impl BinlogFilterRules {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn include(
        mut self,
        database: impl Into<String>,
        table: Option<impl Into<String>>,
    ) -> Self {
        self.rules.push(BinlogFilterRule::Include {
            database: database.into(),
            table: table.map(|s| s.into()),
        });
        self
    }

    pub fn exclude(mut self, database: &str, table: Option<&str>) -> Self {
        self.rules.push(BinlogFilterRule::Exclude {
            database: database.into(),
            table: table.map(|s| s.into()),
        });
        self
    }

    fn eval(&self, event: &Event) -> bool {
        self.rules.iter().fold(true, |st, rule| match rule {
            BinlogFilterRule::Include { database, table } => {
                let Some(Value::String(d)) = event.value.pointer("/meta/database") else {
                    return st;
                };
                if d != database {
                    return st;
                }

                let Some(table) = table else {
                    return true;
                };

                match event.value.pointer("/sql/table") {
                    Some(Value::String(t)) if t == table => true,
                    _ => st,
                }
            }
            BinlogFilterRule::Exclude { database, table } => {
                let Some(Value::String(d)) = event.value.pointer("/meta/database") else {
                    return st;
                };
                if d != database {
                    return st;
                }

                let Some(table) = table else {
                    return false;
                };

                match event.value.pointer("/sql/table") {
                    Some(Value::String(t)) if t == table => false,
                    _ => st,
                }
            }
        })
    }
}
