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
    #[must_use]
    pub fn new(path: &str) -> Self {
        let (pathname, hash, query) = split_path(path);
        let search = query.map_or(serde_json::Value::Null, |q| parse_query_string(&q));
        Self {
            pathname,
            search,
            hash,
            state: serde_json::Value::Null,
        }
    }

    /// Build a location from explicit parts. `hash` may or may not include the
    /// leading `#`.
    #[must_use]
    pub fn from_parts(
        pathname: impl Into<String>,
        search: serde_json::Value,
        hash: impl Into<String>,
    ) -> Self {
        let mut hash: String = hash.into();
        if let Some(stripped) = hash.strip_prefix('#') {
            hash = stripped.to_string();
        }
        Self {
            pathname: pathname.into(),
            search,
            hash,
            state: serde_json::Value::Null,
        }
    }

    /// Parse a full URL into a location.
    ///
    /// # Errors
    /// Returns the underlying [`url::ParseError`] when `url` is not a valid URL.
    pub fn from_url(url: &str) -> Result<Self, url::ParseError> {
        let parsed = url::Url::parse(url)?;
        let query_pairs: HashMap<String, String> = parsed.query_pairs().into_owned().collect();
        // Keep "no query string" distinguishable from "?": only the former
        // maps to JSON null.
        let search = if parsed.query().is_none() {
            serde_json::Value::Null
        } else {
            serde_json::to_value(query_pairs).unwrap_or(serde_json::Value::Null)
        };
        Ok(Self {
            pathname: parsed.path().to_string(),
            search,
            hash: parsed
                .fragment()
                .map(|s| format!("#{s}"))
                .unwrap_or_default(),
            state: serde_json::Value::Null,
        })
    }

    #[must_use]
    pub fn to_url(&self, base: &str) -> String {
        if let Ok(mut url) = url::Url::parse(base) {
            url.set_path(&self.pathname);
            if let serde_json::Value::Object(map) = &self.search {
                let mut query = url.query_pairs_mut();
                for (k, v) in map {
                    append_search_value(&mut query, k, v);
                }
            }
            if !self.hash.is_empty() {
                let frag = self.hash.strip_prefix('#').unwrap_or(&self.hash);
                url.set_fragment(Some(frag));
            }
            url.to_string()
        } else {
            self.pathname.clone()
        }
    }
}

fn split_path(path: &str) -> (String, String, Option<String>) {
    let (before_hash, hash) = match path.split_once('#') {
        Some((p, h)) => (p, h.to_string()),
        None => (path, String::new()),
    };
    let (pathname, query) = match before_hash.split_once('?') {
        Some((p, q)) => (p.to_string(), Some(q.to_string())),
        None => (before_hash.to_string(), None),
    };
    (pathname, hash, query)
}

fn append_search_value<T: url::form_urlencoded::Target>(
    query: &mut url::form_urlencoded::Serializer<'_, T>,
    key: &str,
    value: &serde_json::Value,
) {
    // `append_pair` returns the serializer for chaining; discard it here.
    if let Some(s) = value.as_str() {
        query.append_pair(key, s);
        return;
    }
    if let Some(b) = value.as_bool() {
        query.append_pair(key, &b.to_string());
        return;
    }
    if let Some(n) = value.as_number() {
        query.append_pair(key, &n.to_string());
    }
}

fn coerce_scalar(value: &str) -> serde_json::Value {
    // Fast path: numeric-looking values (digit or leading `-`) skip the
    // boolean checks; everything else keeps the exact original semantics —
    // including the f64 fallback that maps `NaN`/`inf` to JSON null.
    match value.as_bytes().first() {
        None => return serde_json::Value::Null,
        Some(&b) if b.is_ascii_digit() || b == b'-' => {
            if let Ok(i) = value.parse::<i64>() {
                return serde_json::json!(i);
            }
            if let Ok(f) = value.parse::<f64>() {
                return serde_json::json!(f);
            }
        }
        _ => {}
    }
    if let Ok(i) = value.parse::<i64>() {
        return serde_json::json!(i);
    }
    if let Ok(f) = value.parse::<f64>() {
        return serde_json::json!(f);
    }
    if value.eq_ignore_ascii_case("true") {
        return serde_json::Value::Bool(true);
    }
    if value.eq_ignore_ascii_case("false") {
        return serde_json::Value::Bool(false);
    }
    serde_json::Value::String(value.to_string())
}

/// Parse a raw query string into a JSON object.
///
/// Values are percent-decoded, and scalar strings are coerced to numbers or
/// booleans when possible. Repeated keys are collected into JSON arrays.
fn parse_query_string(query: &str) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
        let key = key.into_owned();
        let parsed = coerce_scalar(&value);
        match map.get_mut(&key) {
            None => {
                map.insert(key, parsed);
            }
            Some(existing) => match existing {
                serde_json::Value::Array(arr) => arr.push(parsed),
                other => {
                    let previous = other.take();
                    *other = serde_json::Value::Array(vec![previous, parsed]);
                }
            },
        }
    }
    serde_json::Value::Object(map)
}

/// Deserialize a raw query string into a strongly typed search-params struct,
/// including nested objects and indexed arrays via bracket notation
/// (`filter[status]=open&filter[page]=3&tags[0]=rust`).
///
/// Only available with the `nested-search` feature.
///
/// # Errors
/// Returns [`serde_qs::Error`] when the query string does not deserialize
/// into `T`.
#[cfg(feature = "nested-search")]
pub fn parse_query_nested<T: serde::de::DeserializeOwned>(
    query: &str,
) -> Result<T, serde_qs::Error> {
    serde_qs::from_str(query)
}

/// Serialize typed search params back into a raw query string (no leading
/// `?`), preserving nesting via bracket notation.
///
/// Only available with the `nested-search` feature.
///
/// # Errors
/// Returns [`serde_qs::Error`] when `value` cannot be represented as
/// querystring pairs.
#[cfg(feature = "nested-search")]
pub fn query_to_string<T: serde::Serialize>(value: &T) -> Result<String, serde_qs::Error> {
    serde_qs::to_string(value)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NavigateOptions {
    pub replace: bool,
    pub reset_scroll: Option<bool>,
    pub hash_scroll_into_view: Option<ScrollIntoViewOptions>,
    pub view_transition: Option<ViewTransitionOptions>,
    pub ignore_blocker: bool,
    pub reload_document: bool,
    pub href: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScrollIntoViewOptions {
    pub behavior: Option<String>,
    pub block: Option<String>,
    pub inline: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ViewTransitionOptions {
    pub types: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_new_plain_path() {
        let loc = Location::new("/about");
        assert_eq!(loc.pathname, "/about");
        assert_eq!(loc.search, serde_json::Value::Null);
        assert_eq!(loc.hash, "");
        assert_eq!(loc.state, serde_json::Value::Null);
    }

    #[test]
    fn test_new_with_query() {
        let loc = Location::new("/search?q=navi&page=2");
        assert_eq!(loc.pathname, "/search");
        assert_eq!(loc.search["q"], json!("navi"));
        assert_eq!(loc.search["page"], json!(2));
    }

    #[test]
    fn test_query_type_coercion() {
        let loc = Location::new("/x?n=42&f=1.5&yes=true&no=false&empty=&s=abc");
        assert_eq!(loc.search["n"], json!(42));
        assert_eq!(loc.search["f"], json!(1.5));
        assert_eq!(loc.search["yes"], json!(true));
        assert_eq!(loc.search["no"], json!(false));
        assert_eq!(loc.search["empty"], serde_json::Value::Null);
        assert_eq!(loc.search["s"], json!("abc"));
    }

    #[test]
    fn test_query_coercion_f64_quirks_preserved() {
        // `NaN`/`inf` parse as f64 but cannot be JSON numbers: they must keep
        // coercing to null (pins the non-fast-path fallback semantics).
        let loc = Location::new("/x?a=NaN&b=inf&c=-inf&d=%2B1.5&e=-7");
        assert_eq!(loc.search["a"], serde_json::Value::Null);
        assert_eq!(loc.search["b"], serde_json::Value::Null);
        assert_eq!(loc.search["c"], serde_json::Value::Null);
        assert_eq!(loc.search["d"], json!(1.5));
        assert_eq!(loc.search["e"], json!(-7));
        // A literal `+` decodes to a space, so it is NOT a number.
        let loc = Location::new("/x?d=+1.5");
        assert_eq!(loc.search["d"], json!(" 1.5"));
        // An escaped plus is a literal sign: the i64 fallback path.
        let loc = Location::new("/x?v=%2B5");
        assert_eq!(loc.search["v"], json!(5));
        // A leading-dash non-number falls through to a string.
        let loc = Location::new("/x?v=-abc");
        assert_eq!(loc.search["v"], json!("-abc"));
        // Boolean checks are case-insensitive.
        let loc = Location::new("/x?a=TRUE&b=False");
        assert_eq!(loc.search["a"], json!(true));
        assert_eq!(loc.search["b"], json!(false));
    }

    #[test]
    fn test_query_repeated_keys_become_array() {
        let loc = Location::new("/x?tag=a&tag=b&tag=3");
        assert_eq!(loc.search["tag"], json!(["a", "b", 3]));
    }

    #[test]
    fn test_query_percent_decoding() {
        let loc = Location::new("/x?name=Na%20vi%21&plus=a+b");
        assert_eq!(loc.search["name"], json!("Na vi!"));
        assert_eq!(loc.search["plus"], json!("a b"));
    }

    #[test]
    fn test_new_with_hash() {
        let loc = Location::new("/docs#getting-started");
        assert_eq!(loc.pathname, "/docs");
        assert_eq!(loc.hash, "getting-started");
    }

    #[test]
    fn test_new_with_query_and_hash() {
        let loc = Location::new("/docs?page=1#section-2");
        assert_eq!(loc.pathname, "/docs");
        assert_eq!(loc.search["page"], json!(1));
        assert_eq!(loc.hash, "section-2");
    }

    #[test]
    fn test_from_url() {
        let loc = Location::from_url("https://example.com/users/7?tab=info#top").expect("parse");
        assert_eq!(loc.pathname, "/users/7");
        assert_eq!(loc.search["tab"], json!("info"));
        assert_eq!(loc.hash, "#top");
    }

    #[test]
    fn test_from_url_no_query_no_hash() {
        let loc = Location::from_url("https://example.com/").expect("parse");
        assert_eq!(loc.pathname, "/");
        assert_eq!(loc.search, serde_json::Value::Null);
        assert_eq!(loc.hash, "");
    }

    #[test]
    fn test_from_url_invalid() {
        assert!(Location::from_url("not a url \u{1f600}").is_err());
    }

    #[test]
    fn test_to_url_roundtrip() {
        let loc = Location::from_parts(
            "/users",
            json!({"page": 2, "q": "na vi", "active": true}),
            "#top",
        );
        let url = loc.to_url("https://example.com");
        assert!(url.starts_with("https://example.com/users?"));
        assert!(url.contains("page=2"));
        assert!(url.contains("q=na+vi"));
        assert!(url.contains("active=true"));
        assert!(url.ends_with("#top"));
    }

    #[test]
    fn test_to_url_skips_complex_values() {
        let loc = Location::from_parts("/x", json!({"obj": {"a": 1}, "arr": [1], "keep": "v"}), "");
        let url = loc.to_url("https://e.com");
        assert!(url.contains("keep=v"));
        assert!(!url.contains("obj="));
        assert!(!url.contains("arr="));
    }

    #[test]
    fn test_to_url_invalid_base() {
        let loc = Location::new("/x");
        assert_eq!(loc.to_url("::not a base::"), "/x");
    }

    #[test]
    fn test_to_url_hash_without_prefix() {
        let loc = Location::from_parts("/x", serde_json::Value::Null, "frag");
        let url = loc.to_url("https://e.com");
        assert!(url.ends_with("#frag"));
    }

    #[test]
    fn test_from_parts_strips_hash_prefix() {
        let loc = Location::from_parts("/x", serde_json::Value::Null, "#frag");
        assert_eq!(loc.hash, "frag");
    }

    #[test]
    fn test_location_is_serializable() {
        let loc = Location::new("/a?b=1");
        let s = serde_json::to_string(&loc).expect("serialize");
        let back: Location = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(back, loc);
    }

    #[test]
    fn test_options_defaults() {
        let opts = NavigateOptions::default();
        assert!(!opts.replace);
        assert!(!opts.ignore_blocker);
        assert!(!opts.reload_document);
        assert!(opts.reset_scroll.is_none());
        assert!(opts.hash_scroll_into_view.is_none());
        assert!(opts.view_transition.is_none());
        assert!(opts.href.is_none());
    }

    #[test]
    fn test_scroll_into_view_options() {
        let o = ScrollIntoViewOptions {
            behavior: Some("smooth".into()),
            block: Some("start".into()),
            inline: None,
        };
        assert_eq!(o.behavior.as_deref(), Some("smooth"));
        assert_eq!(o.block.as_deref(), Some("start"));
        assert!(o.inline.is_none());
    }

    #[test]
    fn test_view_transition_options() {
        let o = ViewTransitionOptions {
            types: vec!["slide".into()],
        };
        assert_eq!(o.types, vec!["slide".to_string()]);
    }

    #[cfg(feature = "nested-search")]
    mod nested {
        use super::*;

        #[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
        struct Filter {
            status: String,
            page: u32,
        }

        #[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
        struct Search {
            q: String,
            #[serde(default)]
            filter: Option<Filter>,
            #[serde(default)]
            tags: Vec<String>,
        }

        #[test]
        fn test_parse_nested_bracket_params() {
            let parsed: Search = parse_query_nested(
                "q=router&filter[status]=open&filter[page]=3&tags[0]=rust&tags[1]=gpui",
            )
            .expect("parses");
            assert_eq!(parsed.q, "router");
            assert_eq!(
                parsed.filter,
                Some(Filter {
                    status: "open".to_string(),
                    page: 3,
                })
            );
            assert_eq!(parsed.tags, vec!["rust".to_string(), "gpui".to_string()]);
        }

        #[test]
        fn test_parse_flat_and_defaults() {
            let parsed: Search = parse_query_nested("q=alone").expect("parses");
            assert_eq!(
                parsed,
                Search {
                    q: "alone".to_string(),
                    filter: None,
                    tags: Vec::new(),
                }
            );
        }

        #[test]
        fn test_serialize_roundtrip() {
            let value = Search {
                q: "x".to_string(),
                filter: Some(Filter {
                    status: "closed".to_string(),
                    page: 7,
                }),
                tags: vec!["a".to_string()],
            };
            let query = query_to_string(&value).expect("serializes");
            let back: Search = parse_query_nested(&query).expect("reparses");
            assert_eq!(back, value);
        }

        #[test]
        fn test_invalid_params_error() {
            let result: Result<Search, _> = parse_query_nested("q=ok&filter[page]=notanumber");
            assert!(result.is_err());
        }
    }
}
