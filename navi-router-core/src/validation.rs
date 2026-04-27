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

// Feature-gated impls keep existing ones (not shown for brevity, but leave them).

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

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

    #[test]
    fn test_custom_validate_search_works() {
        let raw = HashMap::new();
        let result = MySearch::validate(&raw);
        assert!(result.is_ok());
    }

    #[cfg(feature = "validator")]
    #[test]
    fn test_validator_integration() {
        use validator::Validate;
        use serde::{Serialize, Deserialize};
        #[derive(Debug, Validate, Default, Serialize, Deserialize)]
        struct ValidatedSearch {
            #[validate(range(min = 1, max = 10))]
            page: Option<u32>,
        }
        let raw: HashMap<String, String> = [("page".to_string(), "5".to_string())].into_iter().collect();
        let result = ValidatedSearch::validate(&raw);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().page, Some(5));
    }
}
