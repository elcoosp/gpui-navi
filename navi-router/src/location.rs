use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

/// Represents the current location in the application, including pathname,
/// search parameters, hash fragment, and navigation state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub pathname: String,
    pub search: serde_json::Value,
    pub hash: String,
    pub state: serde_json::Value,
}

impl Location {
    pub fn new(pathname: &str) -> Self {
        Self {
            pathname: pathname.to_string(),
            search: serde_json::Value::Null,
            hash: String::new(),
            state: serde_json::Value::Null,
        }
    }

    pub fn from_url(url: &str) -> Result<Self, url::ParseError> {
        let parsed = Url::parse(url)?;
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
        if let Ok(mut url) = Url::parse(base) {
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

/// Options for navigation, matching TanStack Router's NavigateOptions.
#[derive(Clone, Default)]
pub struct NavigateOptions {
    pub replace: bool,
    pub reset_scroll: Option<bool>,
    pub hash_scroll_into_view: Option<ScrollIntoViewOptions>,
    pub view_transition: Option<ViewTransitionOptions>,
    pub ignore_blocker: bool,
    pub reload_document: bool,
    pub href: Option<String>,
}

#[derive(Clone, Default)]
pub struct ScrollIntoViewOptions {
    pub behavior: Option<String>,
    pub block: Option<String>,
    pub inline: Option<String>,
}

#[derive(Clone, Default)]
pub struct ViewTransitionOptions {
    pub types: Vec<String>,
}
