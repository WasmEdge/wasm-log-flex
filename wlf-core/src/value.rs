use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// `Value` is a json-like nested structure with fields useful for recording a log, such as `TimeStamp` and `Level`
// TODO: create a macro for easier construction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Int(i64),
    UInt(u64),
    Float(f64),
    Boolean(bool),
    TimeStamp(DateTime<Utc>),
    LogLevel(LogLevel),
    String(String),
    Bytes(Vec<u8>),
    Object(BTreeMap<String, Value>),
    Array(Vec<Value>),
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Fatal,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
    Other(String),
}

impl Value {
    /// Looks up a value by a JSON Pointer.
    pub fn pointer(&self, pointer: &str) -> Option<&Value> {
        if pointer.is_empty() {
            return Some(self);
        }
        if !pointer.starts_with('/') {
            return None;
        }
        pointer
            .split('/')
            .skip(1)
            .map(|x| x.replace("~1", "/").replace("~0", "~"))
            .try_fold(self, |target, token| match target {
                Value::Object(map) => map.get(&token),
                Value::Array(list) => parse_index(&token).and_then(|x| list.get(x)),
                _ => None,
            })
    }

    /// Looks up a value by a JSON Pointer and returns a mutable reference to
    /// that value.
    pub fn pointer_mut(&mut self, pointer: &str) -> Option<&mut Value> {
        if pointer.is_empty() {
            return Some(self);
        }
        if !pointer.starts_with('/') {
            return None;
        }
        pointer
            .split('/')
            .skip(1)
            .map(|x| x.replace("~1", "/").replace("~0", "~"))
            .try_fold(self, |target, token| match target {
                Value::Object(map) => map.get_mut(&token),
                Value::Array(list) => parse_index(&token).and_then(move |x| list.get_mut(x)),
                _ => None,
            })
    }

    /// If the `Value` is a String, returns the associated str. Returns None
    /// otherwise.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// If the `Value` is an Object, returns the associated Map. Returns None
    /// otherwise.
    pub fn as_object(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }

    /// If the `Value` is an Object, returns the associated mutable Map.
    /// Returns None otherwise.
    pub fn as_object_mut(&mut self) -> Option<&mut BTreeMap<String, Value>> {
        match self {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }
}

impl<S: Into<String>, const N: usize> From<[(S, Value); N]> for Value {
    fn from(value: [(S, Value); N]) -> Self {
        Value::Object(BTreeMap::from_iter(
            value.into_iter().map(|(k, v)| (k.into(), v)),
        ))
    }
}

impl From<u8> for Value {
    fn from(value: u8) -> Self {
        Value::UInt(value as u64)
    }
}

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Value::UInt(value as u64)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::Int(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Float(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Boolean(value)
    }
}

impl From<DateTime<Utc>> for Value {
    fn from(value: DateTime<Utc>) -> Self {
        Value::TimeStamp(value)
    }
}

impl From<LogLevel> for Value {
    fn from(value: LogLevel) -> Self {
        Value::LogLevel(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

impl<'a> From<&'a str> for Value {
    fn from(value: &'a str) -> Self {
        Value::String(value.to_string())
    }
}

impl From<Vec<u8>> for Value {
    fn from(value: Vec<u8>) -> Self {
        Value::Bytes(value)
    }
}

impl From<Vec<Value>> for Value {
    fn from(value: Vec<Value>) -> Self {
        Value::Array(value)
    }
}

impl From<BTreeMap<String, Value>> for Value {
    fn from(value: BTreeMap<String, Value>) -> Self {
        Value::Object(value)
    }
}

fn parse_index(s: &str) -> Option<usize> {
    if s.starts_with('+') || (s.starts_with('0') && s.len() != 1) {
        return None;
    }
    s.parse().ok()
}
