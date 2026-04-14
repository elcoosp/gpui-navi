use serde::Deserialize;

/// Configuration for the code generator.
#[derive(Debug, Clone, Deserialize)]
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
    /// Load configuration from a file.
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: NaviConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Get the ignore prefix, defaulting to "-".
    pub fn ignore_prefix(&self) -> &str {
        self.route_file_ignore_prefix.as_deref().unwrap_or("-")
    }

    /// Get the index token, defaulting to "index".
    pub fn index_token(&self) -> &str {
        self.index_token.as_deref().unwrap_or("index")
    }
}
