//! Crate-agnostic search parameter validation.

use std::collections::HashMap;
use std::fmt;

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

pub type ValidationResult<T> = Result<T, Vec<ValidationError>>;

pub trait ValidateSearch: Sized {
    fn validate(raw: &HashMap<String, String>) -> ValidationResult<Self>;
    fn to_query(&self) -> HashMap<String, String>;
}

// ----------------------------------------------------------------------------
// Integration with validator crate
// ----------------------------------------------------------------------------
#[cfg(feature = "validator")]
impl<T> ValidateSearch for T
where
    T: serde::de::DeserializeOwned + serde::Serialize + validator::Validate + Default,
{
    fn validate(raw: &HashMap<String, String>) -> ValidationResult<Self> {
        let value = serde_json::to_value(raw).map_err(|e| {
            vec![ValidationError {
                field: None,
                message: format!("Failed to serialize raw params: {}", e),
            }]
        })?;
        let instance: T = serde_json::from_value(value).map_err(|e| {
            vec![ValidationError {
                field: None,
                message: format!("Deserialization error: {}", e),
            }]
        })?;
        instance.validate().map_err(|errs| {
            errs.field_errors()
                .into_iter()
                .flat_map(|(field, errors)| {
                    errors.iter().map(move |err| ValidationError {
                        field: Some(field.to_string()),
                        message: err.to_string(),
                    })
                })
                .collect::<Vec<_>>()
        })?;
        Ok(instance)
    }

    fn to_query(&self) -> HashMap<String, String> {
        serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_object().map(obj_to_query))
            .unwrap_or_default()
    }
}

// ----------------------------------------------------------------------------
// Integration with validify crate
// ----------------------------------------------------------------------------
#[cfg(feature = "validify")]
impl<T> ValidateSearch for T
where
    T: serde::de::DeserializeOwned + serde::Serialize + validify::Validate + Default,
{
    fn validate(raw: &HashMap<String, String>) -> ValidationResult<Self> {
        let value = serde_json::to_value(raw).map_err(|e| {
            vec![ValidationError {
                field: None,
                message: format!("Failed to serialize raw params: {}", e),
            }]
        })?;
        let instance: T = serde_json::from_value(value).map_err(|e| {
            vec![ValidationError {
                field: None,
                message: format!("Deserialization error: {}", e),
            }]
        })?;
        instance.validate().map_err(|errs| {
            errs.errors()
                .into_iter()
                .map(|err| ValidationError {
                    field: err.field_name().map(|s| s.to_string()),
                    message: err.to_string(),
                })
                .collect::<Vec<_>>()
        })?;
        Ok(instance)
    }

    fn to_query(&self) -> HashMap<String, String> {
        serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_object().map(obj_to_query))
            .unwrap_or_default()
    }
}

// ----------------------------------------------------------------------------
// Integration with valico crate (JSON Schema validation)
// ----------------------------------------------------------------------------
#[cfg(feature = "valico")]
impl<T> ValidateSearch for T
where
    T: serde::de::DeserializeOwned + serde::Serialize + Default + schemars::JsonSchema,
{
    fn validate(raw: &HashMap<String, String>) -> ValidationResult<Self> {
        let value = serde_json::to_value(raw).map_err(|e| {
            vec![ValidationError {
                field: None,
                message: format!("Failed to serialize raw params: {}", e),
            }]
        })?;

        let mut generator = schemars::SchemaGenerator::default();
        let schema = T::json_schema(&mut generator);
        let schema_json = serde_json::to_value(&schema).unwrap();

        let mut scope = valico::json_schema::Scope::new();
        let compiled_schema = scope.compile_and_return(schema_json, false).map_err(|e| {
            vec![ValidationError {
                field: None,
                message: format!("Schema compilation error: {}", e),
            }]
        })?;

        let state = compiled_schema.validate(&value);
        if !state.is_valid() {
            let errors = state
                .errors
                .into_iter()
                .map(|e| ValidationError {
                    field: Some(e.get_path().into()),
                    message: e.get_title().to_string(),
                })
                .collect();
            return Err(errors);
        }

        let instance: T = serde_json::from_value(value).map_err(|e| {
            vec![ValidationError {
                field: None,
                message: format!("Deserialization error: {}", e),
            }]
        })?;
        Ok(instance)
    }

    fn to_query(&self) -> HashMap<String, String> {
        serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_object().map(obj_to_query))
            .unwrap_or_default()
    }
}

pub trait SearchMiddleware: Send + Sync {
    fn transform(&self, search: serde_json::Value) -> serde_json::Value;
}

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

pub fn to_query_string(params: &HashMap<String, String>) -> String {
    let mut parts: Vec<String> = params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    parts.sort();
    parts.join("&")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_validate_search_custom() {
        #[derive(Debug, Default)]
        struct MySearch;
        impl ValidateSearch for MySearch {
            fn validate(_raw: &HashMap<String, String>) -> ValidationResult<Self> {
                Ok(MySearch)
            }
            fn to_query(&self) -> HashMap<String, String> {
                HashMap::new()
            }
        }
        let raw = HashMap::new();
        let result = MySearch::validate(&raw);
        assert!(result.is_ok());
    }
}
