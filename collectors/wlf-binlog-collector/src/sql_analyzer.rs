use sqlparser::{
    ast::Statement,
    dialect::MySqlDialect,
    parser::{Parser, ParserError},
};
use thiserror::Error;
use wlf_core::{value, Value};

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to parse sql, {0}")]
    Parse(#[from] ParserError),
    #[error("{0}")]
    Other(String),
}

pub(super) struct SqlAnalyzer;

impl SqlAnalyzer {
    pub(super) fn new() -> Self {
        Self
    }

    pub(super) fn analyze(&self, sql: &str) -> Result<Value, Error> {
        // parse
        let mut ast = Parser::parse_sql(&MySqlDialect {}, sql)?;
        if ast.len() != 1 {
            return Err(Error::Other("multiple statements in one sql".to_string()));
        }
        let st = ast.remove(0);

        // extract info
        let properties = match st {
            Statement::Insert { table_name, .. } => {
                value!({
                    "type": "insert",
                    "table": table_name.to_string()
                })
            }
            Statement::CreateTable { name, .. } => {
                value!({
                    "type": "table-create",
                    "table": name.to_string()
                })
            }
            _ => value!({}),
        };

        Ok(properties)
    }
}
