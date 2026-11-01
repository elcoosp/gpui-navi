use serde::Deserialize;

/// Configuration for the route-tree code generator.
///
/// All fields fall back to their defaults when missing from the JSON file,
/// so a minimal `navi.config.json` only needs the two paths.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct NaviConfig {
    /// Directory containing route files.
    pub routes_directory: String,
    /// Output path for the generated route tree module.
    pub generated_route_tree: String,
    /// Token used to identify route files.
    pub route_token: Option<String>,
    /// Token used to identify index routes.
    pub index_token: Option<String>,
    /// Prefix for files/folders that should be ignored.
    pub route_file_ignore_prefix: Option<String>,
}

impl Default for NaviConfig {
    fn default() -> Self {
        Self {
            routes_directory: "./src/routes".to_string(),
            generated_route_tree: "./src/route_tree.gen.rs".to_string(),
            route_token: Some("route".to_string()),
            index_token: Some("index".to_string()),
            route_file_ignore_prefix: Some("-".to_string()),
        }
    }
}

impl NaviConfig {
    /// Load configuration from a JSON file.
    ///
    /// # Errors
    /// Returns an error when the file cannot be read or is not valid JSON
    /// with the expected fields.
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: NaviConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Discover `navi.config.json` by walking up from `start_dir` (inclusive).
    /// Returns the default config when nothing is found.
    ///
    /// # Errors
    /// Returns an error when a candidate config exists but cannot be parsed.
    pub fn discover(start_dir: &std::path::Path) -> anyhow::Result<Self> {
        for dir in start_dir.ancestors() {
            let candidate = dir.join("navi.config.json");
            if candidate.is_file() {
                return Self::from_file(&candidate.to_string_lossy());
            }
        }
        Ok(Self::default())
    }

    /// Get the ignore prefix, defaulting to `-`.
    #[must_use]
    pub fn ignore_prefix(&self) -> &str {
        self.route_file_ignore_prefix.as_deref().unwrap_or("-")
    }

    /// Get the index token, defaulting to `index`.
    #[must_use]
    pub fn index_token(&self) -> &str {
        self.index_token.as_deref().unwrap_or("index")
    }

    /// Get the route token, defaulting to `route`.
    #[must_use]
    pub fn route_token(&self) -> &str {
        self.route_token.as_deref().unwrap_or("route")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let config = NaviConfig::default();
        assert_eq!(config.routes_directory, "./src/routes");
        assert_eq!(config.generated_route_tree, "./src/route_tree.gen.rs");
        assert_eq!(config.ignore_prefix(), "-");
        assert_eq!(config.index_token(), "index");
        assert_eq!(config.route_token(), "route");
    }

    #[test]
    fn test_accessors_fall_back_when_none() {
        let config = NaviConfig {
            route_token: None,
            index_token: None,
            route_file_ignore_prefix: None,
            ..NaviConfig::default()
        };
        assert_eq!(config.ignore_prefix(), "-");
        assert_eq!(config.index_token(), "index");
        assert_eq!(config.route_token(), "route");
    }

    #[test]
    fn test_deserialize_partial_config() {
        let config: NaviConfig = serde_json::from_str(
            r#"{"routes_directory": "./r", "generated_route_tree": "./t.rs"}"#,
        )
        .expect("partial config");
        assert_eq!(config.routes_directory, "./r");
        assert!(config.route_token.is_some(), "serde defaults fill the rest");
    }

    #[test]
    fn test_deserialize_empty_config_uses_defaults() {
        let config: NaviConfig = serde_json::from_str("{}").expect("empty config");
        assert_eq!(config, NaviConfig::default());
    }

    #[test]
    fn test_deserialize_invalid_type_rejected() {
        let result: Result<NaviConfig, _> = serde_json::from_str(r#"{"routes_directory": 42}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_file_missing_errors() {
        assert!(NaviConfig::from_file("/nope/navi.config.json").is_err());
    }

    #[test]
    fn test_from_file_invalid_json_errors() {
        let path = std::env::temp_dir().join("navi-config-invalid.json");
        std::fs::write(&path, "{ not json }").expect("write");
        let result = NaviConfig::from_file(&path.to_string_lossy());
        let _ = std::fs::remove_file(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_file_valid() {
        let path = std::env::temp_dir().join("navi-config-valid.json");
        std::fs::write(
            &path,
            r#"{"routes_directory": "./routes", "generated_route_tree": "./gen.rs"}"#,
        )
        .expect("write");
        let config = NaviConfig::from_file(&path.to_string_lossy()).expect("ok");
        let _ = std::fs::remove_file(&path);
        assert_eq!(config.routes_directory, "./routes");
    }

    #[test]
    fn test_discover_finds_config_upwards() {
        let base = std::env::temp_dir().join(format!("navi-discover-{}", std::process::id()));
        let nested = base.join("a/b/c");
        std::fs::create_dir_all(&nested).expect("mkdir");
        std::fs::write(
            base.join("navi.config.json"),
            r#"{"routes_directory": "./found"}"#,
        )
        .expect("write");
        let config = NaviConfig::discover(&nested).expect("ok");
        let _ = std::fs::remove_dir_all(&base);
        assert_eq!(config.routes_directory, "./found");
    }

    #[test]
    fn test_discover_falls_back_to_default() {
        let config = NaviConfig::discover(std::path::Path::new("/")).expect("ok");
        assert_eq!(config.routes_directory, "./src/routes");
    }
}
