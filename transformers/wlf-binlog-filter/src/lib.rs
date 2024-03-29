use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use thiserror::Error;
use tracing::info;
use wlf_core::{
    event_router::{EventRouter, EventRouterApi},
    ComponentApi, ComponentKind, Event, Value,
};

#[derive(Deserialize, Debug)]
pub struct BinlogFilter {
    pub id: String,
    pub destination: String,
    #[serde(flatten)]
    pub rules: BinlogFilterRules,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("event router error, {0}")]
    EventRouter(#[from] wlf_core::event_router::Error),
}

#[async_trait]
impl ComponentApi for BinlogFilter {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn kind(&self) -> ComponentKind {
        ComponentKind::Transformer
    }

    async fn run(&self, router: Arc<EventRouter>) -> Result<(), Box<dyn std::error::Error>> {
        while let Ok(event) = router.poll_event(self.id()).await {
            info!("{} receives new event:\n\t{event:?}", self.id);

            if !self.rules.eval(&event) {
                continue;
            }

            router.send_event(event, self.destination.as_str()).await?;
        }
        Ok(())
    }
}

#[derive(Default, Deserialize, Debug)]
pub struct BinlogFilterRules {
    rules: Vec<BinlogFilterRule>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum BinlogFilterRule {
    Include { database: String, table: String },
    Exclude { database: String, table: String },
}

impl BinlogFilterRules {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn include(&mut self, database: impl Into<String>, table: impl Into<String>) {
        self.rules.push(BinlogFilterRule::Include {
            database: database.into(),
            table: table.into(),
        });
    }

    pub fn exclude(&mut self, database: impl Into<String>, table: impl Into<String>) {
        self.rules.push(BinlogFilterRule::Exclude {
            database: database.into(),
            table: table.into(),
        });
    }

    fn eval(&self, event: &Event) -> bool {
        self.rules.iter().fold(true, |st, rule| match rule {
            BinlogFilterRule::Include { database, table } => {
                let Some(Value::String(d)) = event.value.pointer("/database") else {
                    return st;
                };
                if database == "*" {
                    return true;
                }

                if d != database {
                    return st;
                }

                if table == "*" {
                    return true;
                }

                match event.value.pointer("/table") {
                    Some(Value::String(t)) if t == table => true,
                    _ => st,
                }
            }
            BinlogFilterRule::Exclude { database, table } => {
                let Some(Value::String(d)) = event.value.pointer("/database") else {
                    return st;
                };

                if database == "*" {
                    return false;
                }

                if d != database {
                    return st;
                }

                if table == "*" {
                    return false;
                }

                match event.value.pointer("/table") {
                    Some(Value::String(t)) if t == table => false,
                    _ => st,
                }
            }
        })
    }
}
