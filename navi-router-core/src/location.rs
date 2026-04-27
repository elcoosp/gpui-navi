use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub pathname: String,
    pub search: serde_json::Value,
    pub hash: String,
    pub state: serde_json::Value,
}

impl Location {
    pub fn new(path: &str) -> Self {
        let (pathname, query) = match path.split_once('?') {
            Some((p, q)) => (p.to_string(), Some(q.to_string())),
            None => (path.to_string(), None),
        };
        let search = if let Some(q) = query {
            parse_query_string(&q)
        } else {
            serde_json::Value::Null
        };
        Self {
            pathname,
            search,
            hash: String::new(),
            state: serde_json::Value::Null,
        }
    }

    pub fn from_url(url: &str) -> Result<Self, url::ParseError> {
        let parsed = url::Url::parse(url)?;
        let query_pairs: HashMap<String, String> = parsed.query_pairs().into_owned().collect();
        let search = serde_json::to_value(query_pairs).unwrap_or(serde_json::Value::Null);
        Ok(Self {
            pathname: parsed.path().to_string(),
            search,
            hash: parsed
                .fragment()
                .map(|s| format!("#{}", s))
                .unwrap_or_default(),
            state: serde_json::Value::Null,
        })
    }

    pub fn to_url(&self, base: &str) -> String {
        if let Ok(mut url) = url::Url::parse(base) {
            url.set_path(&self.pathname);
            if let serde_json::Value::Object(map) = &self.search {
                let mut query = url.query_pairs_mut();
                for (k, v) in map {
                    if let Some(s) = v.as_str() {
                        query.append_pair(k, s);
                    }
                }
            }
            if !self.hash.is_empty() && self.hash.starts_with('#') {
                url.set_fragment(Some(&self.hash[1..]));
            }
            url.to_string()
        } else {
            self.pathname.clone()
        }
    }
}

fn parse_query_string(query: &str) -> serde_json::Value {
    let map: serde_json::Map<String, serde_json::Value> = query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.split('=');
            let key = parts.next()?.to_string();
            let value = parts.next().unwrap_or("");
            let parsed = if value.is_empty() {
                serde_json::Value::Null
            } else if let Ok(i) = value.parse::<i64>() {
                serde_json::json!(i)
            } else if let Ok(f) = value.parse::<f64>() {
                serde_json::json!(f)
            } else if value.eq_ignore_ascii_case("true") {
                serde_json::Value::Bool(true)
            } else if value.eq_ignore_ascii_case("false") {
                serde_json::Value::Bool(false)
            } else {
                serde_json::Value::String(value.to_string())
            };
            Some((key, parsed))
        })
        .collect();
    serde_json::Value::Object(map)
}

#[derive(Clone, Default, PartialEq)]
pub struct NavigateOptions {
    pub replace: bool,
    pub reset_scroll: Option<bool>,
    pub hash_scroll_into_view: Option<ScrollIntoViewOptions>,
    pub view_transition: Option<ViewTransitionOptions>,
    pub ignore_blocker: bool,
    pub reload_document: bool,
    pub href: Option<String>,
}

#[derive(Clone, Default, PartialEq)]
pub struct ScrollIntoViewOptions {
    pub behavior: Option<String>,
    pub block: Option<String>,
    pub inline: Option<String>,
}

#[derive(Clone, Default, PartialEq)]
pub struct ViewTransitionOptions {
    pub types: Vec<String>,
}
