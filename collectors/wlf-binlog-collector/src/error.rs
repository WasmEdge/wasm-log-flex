use thiserror::Error;

use crate::sql_analyzer;

#[derive(Error, Debug)]
pub enum Error {
    #[error("binlog client error, {0}")]
    BinlogClient(#[from] mysql_cdc::errors::Error),
    #[error("event hub error, {0}")]
    EventHub(#[from] wlf_core::event_hub::Error),
    #[error("failed to analyze sql statement, {0}")]
    SqlAnalyzer(#[from] sql_analyzer::Error),
    #[error("{0}")]
    Other(String),
}
