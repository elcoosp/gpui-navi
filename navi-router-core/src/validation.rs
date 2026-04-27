use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: Option<String>,
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(fld) = &self.field {
            write!(f, "{}: {}", fld, self.message)
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

// Utility to convert a JSON object into query params
fn obj_to_query(obj: &serde_json::Map<String, serde_json::Value>) -> HashMap<String, String> {
    obj.iter()
        .filter_map(|(k, v)| {
            v.as_str().map(|s| (k.clone(), s.to_string()))
                .or_else(|| Some((k.clone(), v.to_string())))
        })
        .collect()
}
