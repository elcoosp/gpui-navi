//! Crate-agnostic search parameter validation.
//!
//! The [`ValidateSearch`] trait turns a raw `HashMap<String, String>` (URL
//! search params) into a typed, validated struct. Integrations are provided
//! for the `validator`, `garde`, `validify` and `valico` (JSON Schema) crates
//! behind their respective cargo features.

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
            write!(f, "{field}: {}", self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for ValidationError {}

pub type ValidationResult<T> = Result<T, Vec<ValidationError>>;

pub trait ValidateSearch: Sized {
    /// Validate raw search params, producing a typed value or a list of errors.
    ///
    /// # Errors
    /// Returns every validation failure (deserialization and rule violations)
    /// as a list of [`ValidationError`]s.
    fn validate(raw: &HashMap<String, String>) -> ValidationResult<Self>;
    /// Serialize back into query parameters.
    fn to_query(&self) -> HashMap<String, String>;
}

#[cfg(any(
    feature = "validator",
    feature = "garde",
    feature = "validify",
    feature = "valico"
))]
fn single_error(message: impl Into<String>) -> Vec<ValidationError> {
    vec![ValidationError {
        field: None,
        message: message.into(),
    }]
}

#[cfg(any(
    feature = "validator",
    feature = "garde",
    feature = "validify",
    feature = "valico"
))]
fn raw_to_json(raw: &HashMap<String, String>) -> Result<serde_json::Value, Vec<ValidationError>> {
    serde_json::to_value(raw)
        .map_err(|e| single_error(format!("Failed to serialize raw params: {e}")))
}

// Used by the deserializing integrations (validator/garde/validify); valico
// validates the raw JSON value against a schema instead.
#[cfg(any(feature = "validator", feature = "garde", feature = "validify"))]
fn deserialize_from_raw<T: serde::de::DeserializeOwned>(
    raw: &HashMap<String, String>,
) -> Result<T, Vec<ValidationError>> {
    let value = raw_to_json(raw)?;
    serde_json::from_value(value).map_err(|e| single_error(format!("Deserialization error: {e}")))
}

#[cfg(any(
    feature = "validator",
    feature = "garde",
    feature = "validify",
    feature = "valico"
))]
fn json_obj_to_query(obj: &serde_json::Map<String, serde_json::Value>) -> HashMap<String, String> {
    obj.iter()
        .filter_map(|(k, v)| {
            let s = match v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Bool(_) | serde_json::Value::Number(_) => v.to_string(),
                _ => return None,
            };
            Some((k.clone(), s))
        })
        .collect()
}

#[cfg(any(
    feature = "validator",
    feature = "garde",
    feature = "validify",
    feature = "valico"
))]
fn to_query_via_json<T: serde::Serialize>(value: &T) -> HashMap<String, String> {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_object().map(json_obj_to_query))
        .unwrap_or_default()
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
        let instance: Self = deserialize_from_raw(raw)?;
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
        to_query_via_json(self)
    }
}

// ----------------------------------------------------------------------------
// Integration with garde crate
// ----------------------------------------------------------------------------
#[cfg(feature = "garde")]
impl<T> ValidateSearch for T
where
    T: serde::de::DeserializeOwned + serde::Serialize + garde::Validate + Default,
    T::Context: Default,
{
    fn validate(raw: &HashMap<String, String>) -> ValidationResult<Self> {
        let instance: Self = deserialize_from_raw(raw)?;
        instance.validate().map_err(|report| {
            report
                .iter()
                .map(|(path, err)| ValidationError {
                    field: Some(path.to_string()),
                    message: err.to_string(),
                })
                .collect::<Vec<_>>()
        })?;
        Ok(instance)
    }

    fn to_query(&self) -> HashMap<String, String> {
        to_query_via_json(self)
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
        let instance: Self = deserialize_from_raw(raw)?;
        instance.validate().map_err(|errs| {
            errs.errors()
                .iter()
                .map(|err| ValidationError {
                    field: err.field_name().map(str::to_string),
                    message: err.to_string(),
                })
                .collect::<Vec<_>>()
        })?;
        Ok(instance)
    }

    fn to_query(&self) -> HashMap<String, String> {
        to_query_via_json(self)
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
        let value = raw_to_json(raw)?;

        let schema = schemars::schema_for!(T);
        let schema_json = serde_json::to_value(&schema)
            .map_err(|e| single_error(format!("Schema serialization error: {e}")))?;

        let mut scope = valico::json_schema::Scope::new();
        let compiled_schema = scope
            .compile_and_return(schema_json, false)
            .map_err(|e| single_error(format!("Schema compilation error: {e}")))?;

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

        let instance: Self = serde_json::from_value(value)
            .map_err(|e| single_error(format!("Deserialization error: {e}")))?;
        Ok(instance)
    }

    fn to_query(&self) -> HashMap<String, String> {
        to_query_via_json(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct MySearch {
        page: u32,
    }

    impl ValidateSearch for MySearch {
        fn validate(_raw: &HashMap<String, String>) -> ValidationResult<Self> {
            Ok(MySearch { page: 1 })
        }

        fn to_query(&self) -> HashMap<String, String> {
            let mut m = HashMap::new();
            m.insert("page".to_string(), "1".to_string());
            m
        }
    }

    #[test]
    fn test_custom_validate_search_works() {
        let raw = HashMap::new();
        let result = MySearch::validate(&raw).expect("ok");
        assert_eq!(result.page, 1);
        assert_eq!(result.to_query().get("page").map(String::as_str), Some("1"));
    }

    #[test]
    fn test_validation_error_display_with_field() {
        let e = ValidationError {
            field: Some("page".into()),
            message: "out of range".into(),
        };
        assert_eq!(e.to_string(), "page: out of range");
    }

    #[test]
    fn test_validation_error_display_without_field() {
        let e = ValidationError {
            field: None,
            message: "boom".into(),
        };
        assert_eq!(e.to_string(), "boom");
    }

    #[test]
    fn test_validation_error_is_std_error() {
        let e = ValidationError {
            field: None,
            message: "x".into(),
        };
        let boxed: Box<dyn std::error::Error> = Box::new(e);
        assert_eq!(boxed.to_string(), "x");
    }

    #[cfg(any(
        feature = "validator",
        feature = "garde",
        feature = "validify",
        feature = "valico"
    ))]
    #[test]
    fn test_json_obj_to_query_primitives_only() {
        let obj = serde_json::json!({
            "s": "v",
            "n": 3,
            "b": true,
            "nested": {"x": 1},
            "arr": [1],
            "nul": null
        });
        let q = json_obj_to_query(obj.as_object().expect("obj"));
        assert_eq!(q.get("s").map(String::as_str), Some("v"));
        assert_eq!(q.get("n").map(String::as_str), Some("3"));
        assert_eq!(q.get("b").map(String::as_str), Some("true"));
        assert!(!q.contains_key("nested"));
        assert!(!q.contains_key("arr"));
        assert!(!q.contains_key("nul"));
    }

    #[cfg(any(
        feature = "validator",
        feature = "garde",
        feature = "validify",
        feature = "valico"
    ))]
    #[test]
    fn test_to_query_via_json_roundtrip() {
        #[derive(serde::Serialize)]
        struct S {
            a: u32,
            b: String,
        }
        let q = to_query_via_json(&S {
            a: 1,
            b: "x".into(),
        });
        assert_eq!(q.get("a").map(String::as_str), Some("1"));
        assert_eq!(q.get("b").map(String::as_str), Some("x"));
    }

    // `deserialize_from_raw` only exists for the deserializing integrations;
    // valico validates the raw JSON value against a schema instead.
    #[cfg(any(feature = "validator", feature = "garde", feature = "validify"))]
    #[test]
    fn test_deserialize_from_raw_error_message() {
        let raw: HashMap<String, String> = HashMap::new();
        let err = deserialize_from_raw::<u32>(&raw).expect_err("must fail");
        assert_eq!(err.len(), 1);
        assert!(err[0].message.contains("Deserialization error"));
    }

    #[cfg(feature = "validator")]
    mod validator_tests {
        use super::*;
        use serde::{Deserialize, Serialize};
        use validator::Validate;

        // Search params arrive as strings; rules run on the string itself.
        #[derive(Debug, Validate, Default, Serialize, Deserialize)]
        struct ValidatedSearch {
            #[validate(length(min = 1, max = 10))]
            page: Option<String>,
        }

        #[test]
        fn test_validator_ok() {
            let raw: HashMap<String, String> = [("page".to_string(), "5".to_string())]
                .into_iter()
                .collect();
            let result = <ValidatedSearch as ValidateSearch>::validate(&raw).expect("valid");
            assert_eq!(result.page.as_deref(), Some("5"));
        }

        #[test]
        fn test_validator_rejects_too_long() {
            let raw: HashMap<String, String> = [("page".to_string(), "12345678901".to_string())]
                .into_iter()
                .collect();
            let err = <ValidatedSearch as ValidateSearch>::validate(&raw).expect_err("invalid");
            assert_eq!(err.len(), 1);
            assert_eq!(err[0].field.as_deref(), Some("page"));
        }

        #[test]
        fn test_validator_missing_field_deserializes_to_default() {
            let raw: HashMap<String, String> = HashMap::new();
            let result = <ValidatedSearch as ValidateSearch>::validate(&raw).expect("valid");
            assert_eq!(result.page, None);
        }

        #[test]
        fn test_validator_to_query() {
            let s = ValidatedSearch {
                page: Some("3".to_string()),
            };
            let q = s.to_query();
            assert_eq!(q.get("page").map(String::as_str), Some("3"));
        }
    }

    #[cfg(feature = "garde")]
    mod garde_tests {
        use super::*;
        use garde::Validate;
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Validate, Default, Serialize, Deserialize)]
        struct GardeSearch {
            #[garde(ascii, length(min = 1, max = 10))]
            page: Option<String>,
        }

        #[test]
        fn test_garde_ok() {
            let raw: HashMap<String, String> = [("page".to_string(), "5".to_string())]
                .into_iter()
                .collect();
            let result = <GardeSearch as ValidateSearch>::validate(&raw).expect("valid");
            assert_eq!(result.page.as_deref(), Some("5"));
        }

        #[test]
        fn test_garde_rejects_non_ascii() {
            let raw: HashMap<String, String> = [("page".to_string(), "h\u{e9}llo".to_string())]
                .into_iter()
                .collect();
            let err = <GardeSearch as ValidateSearch>::validate(&raw).expect_err("invalid");
            assert!(!err.is_empty());
        }

        #[test]
        fn test_garde_to_query() {
            let s = GardeSearch {
                page: Some("3".to_string()),
            };
            let q = s.to_query();
            assert_eq!(q.get("page").map(String::as_str), Some("3"));
        }
    }

    #[cfg(feature = "validify")]
    mod validify_tests {
        use super::*;
        use serde::{Deserialize, Serialize};
        use validify::Validate;

        #[derive(Debug, Validate, Default, Serialize, Deserialize)]
        struct ValidifySearch {
            #[validate(length(min = 1, max = 10))]
            page: Option<String>,
        }

        #[test]
        fn test_validify_ok() {
            let raw: HashMap<String, String> = [("page".to_string(), "5".to_string())]
                .into_iter()
                .collect();
            let result = <ValidifySearch as ValidateSearch>::validate(&raw).expect("valid");
            assert_eq!(result.page.as_deref(), Some("5"));
        }

        #[test]
        fn test_validify_rejects_too_long() {
            let raw: HashMap<String, String> = [("page".to_string(), "12345678901".to_string())]
                .into_iter()
                .collect();
            let err = <ValidifySearch as ValidateSearch>::validate(&raw).expect_err("invalid");
            assert!(!err.is_empty());
        }

        #[test]
        fn test_validify_to_query() {
            let s = ValidifySearch {
                page: Some("3".to_string()),
            };
            let q = s.to_query();
            assert_eq!(q.get("page").map(String::as_str), Some("3"));
        }
    }

    #[cfg(feature = "valico")]
    mod valico_tests {
        use super::*;
        use schemars::JsonSchema;
        use serde::{Deserialize, Serialize};

        // URL search params always arrive as strings; the JSON Schema sees
        // strings, so the schema-typed struct uses `String` fields.
        #[derive(Debug, Default, Serialize, Deserialize, JsonSchema)]
        #[serde(deny_unknown_fields)]
        struct SchemaSearch {
            #[serde(skip_serializing_if = "Option::is_none")]
            page: Option<String>,
        }

        #[test]
        fn test_valico_ok() {
            let raw: HashMap<String, String> = [("page".to_string(), "5".to_string())]
                .into_iter()
                .collect();
            let result = <SchemaSearch as ValidateSearch>::validate(&raw).expect("valid");
            assert_eq!(result.page.as_deref(), Some("5"));
        }

        #[test]
        fn test_valico_rejects_wrong_type() {
            // A number is not a string: schema rejects it.
            let value = serde_json::json!({"page": 5});
            let mut scope = valico::json_schema::Scope::new();
            let schema = serde_json::to_value(schemars::schema_for!(SchemaSearch)).expect("schema");
            let compiled = scope.compile_and_return(schema, false).expect("compile");
            assert!(!compiled.validate(&value).is_valid());
        }

        #[test]
        fn test_valico_to_query() {
            let s = SchemaSearch {
                page: Some("3".to_string()),
            };
            let q = s.to_query();
            assert_eq!(q.get("page").map(String::as_str), Some("3"));
        }
    }
}
