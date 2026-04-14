//! Crate-agnostic search parameter validation.

use std::collections::HashMap;
use std::fmt;

/// A validation error with optional field information.
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: Option<String>,
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(field) = &self.field {
            write!(f, "{}: {}", field, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for ValidationError {}

/// Result type for validation operations.
pub type ValidationResult<T> = Result<T, Vec<ValidationError>>;

/// Core validation trait for search parameters.
/// Implement this trait for types that represent validated search parameters.
pub trait ValidateSearch: Sized {
    /// Validate raw query parameters and produce a typed value.
    fn validate(raw: &HashMap<String, String>) -> ValidationResult<Self>;

    /// Convert the validated value back to query parameters.
    fn to_query(&self) -> HashMap<String, String>;
}

/// A default implementation that accepts any parameters without validation.
impl ValidateSearch for () {
    fn validate(_raw: &HashMap<String, String>) -> ValidationResult<Self> {
        Ok(())
    }

    fn to_query(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

/// Trait for search parameter middleware that can transform search params.
pub trait SearchMiddleware: Send + Sync {
    fn transform(&self, search: serde_json::Value) -> serde_json::Value;
}

/// Middleware that retains only specified search parameter keys.
pub struct RetainSearchParams {
    pub keys: Vec<String>,
}

impl SearchMiddleware for RetainSearchParams {
    fn transform(&self, mut search: serde_json::Value) -> serde_json::Value {
        if let serde_json::Value::Object(map) = &mut search {
            map.retain(|k, _| self.keys.contains(k));
        }
        search
    }
}

/// Middleware that strips search parameters that match default values.
pub struct StripSearchParams {
    pub defaults: serde_json::Value,
}

impl SearchMiddleware for StripSearchParams {
    fn transform(&self, mut search: serde_json::Value) -> serde_json::Value {
        if let (serde_json::Value::Object(map), serde_json::Value::Object(defaults)) =
            (&mut search, &self.defaults)
        {
            map.retain(|k, v| defaults.get(k) != Some(v));
        }
        search
    }
}

/// Helper to convert a HashMap<String, String> to a query string.
pub fn to_query_string(params: &HashMap<String, String>) -> String {
    let mut parts: Vec<String> = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();
    parts.sort();
    parts.join("&")
}
