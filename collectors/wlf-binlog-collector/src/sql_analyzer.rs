use std::collections::HashMap;

use sqlparser::{
    ast::{ColumnDef, Statement},
    dialect::MySqlDialect,
    parser::{Parser, ParserError},
};
use thiserror::Error;
use wlf_core::{value, Value};

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to parse sql, {0}")]
    Parse(#[from] ParserError),
    #[error("table {0} not found")]
    TableIdNotFound(u64),
    #[error("table {1} not found in database {0}")]
    TableNotFound(String, String),
    #[error("{0}")]
    Other(String),
}

/// Database and Table name
pub(crate) type TableRef = (String, String);

pub(crate) struct SqlAnalyzer {
    table_map: HashMap<u64, TableRef>,
    columns_map: HashMap<TableRef, Vec<ColumnDef>>,
}

impl SqlAnalyzer {
    pub(crate) fn new() -> Self {
        Self {
            table_map: HashMap::new(),
            columns_map: HashMap::new(),
        }
    }

    pub(crate) fn analyze(&mut self, database: &str, sql: &str) -> Result<Value, Error> {
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
                    "database" : database,
                    "table": table_name.to_string()
                })
            }
            Statement::CreateTable { name, columns, .. } => {
                let mut value = value!({
                    "type": "table-create",
                    "database" : database,
                    "table": name.to_string(),
                    "columns": {}
                });
                let map = value
                    .pointer_mut("/columns")
                    .unwrap()
                    .as_object_mut()
                    .unwrap();
                for column in &columns {
                    map.insert(column.name.to_string(), column.data_type.to_string().into());
                }
                self.columns_map
                    .insert((database.to_string(), name.to_string()), columns);
                value
            }
            Statement::CreateDatabase { .. } => {
                value!({
                    "database" : database,
                    "type": "database-create",
                })
            }
            // return null if the sql is not worth analyzing. e.g., transaction BEGIN
            _ => value!(null),
        };

        Ok(properties)
    }

    pub(crate) fn map_table(&mut self, database: &str, table: &str, id: u64) {
        self.table_map
            .insert(id, (database.to_string(), table.to_string()));
    }

    pub(crate) fn get_table_info(&self, table_id: u64) -> Result<&TableRef, Error> {
        self.table_map
            .get(&table_id)
            .ok_or(Error::TableIdNotFound(table_id))
    }

    pub(crate) fn get_column_defs(&self, table_id: u64) -> Result<&Vec<ColumnDef>, Error> {
        let table_ref = self
            .table_map
            .get(&table_id)
            .ok_or(Error::TableIdNotFound(table_id))?;
        self.columns_map.get(table_ref).ok_or(Error::TableNotFound(
            table_ref.0.to_owned(),
            table_ref.1.to_owned(),
        ))
    }
}
