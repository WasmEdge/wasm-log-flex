use once_cell::unsync::Lazy;
use regex::{Captures, Regex};
use wlf_core::{Event, Value};

pub mod test_utils;

pub fn substitute_with_event(template: &str, event: &Event) -> Result<String, String> {
    let replacer =
        Lazy::new(|| Regex::new(r"%\{(?P<path>.+?)\}").expect("can't create topic replacer"));
    let mut fail_reason = None;
    let topic_name = replacer
        .replace_all(template, |caps: &Captures| {
            let path = &caps["path"];
            if let Some(Value::String(value)) = event.value.pointer(path) {
                value.to_string()
            } else {
                fail_reason = Some(format!(
                    "no {path} field or {path} is not string, event: {event:?}"
                ));
                String::new()
            }
        })
        .to_string();
    if let Some(reason) = fail_reason {
        Err(reason)
    } else {
        Ok(topic_name)
    }
}
