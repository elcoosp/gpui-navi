use crate::config::NaviConfig;
use anyhow::Result;
use regex::Regex;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Information about a discovered route file.
#[derive(Debug, Clone)]
pub struct RouteInfo {
    /// Relative path from the routes directory.
    pub relative_path: PathBuf,
    /// The route path pattern (e.g., "/users/$id").
    pub route_pattern: String,
    /// The module name for code generation.
    pub module_name: String,
    /// Whether this is a layout route.
    pub is_layout: bool,
    /// Whether this is an index route.
    pub is_index: bool,
    /// Whether this is a root route.
    pub is_root: bool,
    /// Whether this route has a dynamic segment.
    pub has_dynamic_segment: bool,
    /// The parent route's relative path.
    pub parent: Option<String>,
}

/// Scan the routes directory and discover all route files.
pub fn scan_routes(config: &NaviConfig) -> Result<Vec<RouteInfo>> {
    let routes_dir = Path::new(&config.routes_directory);
    if !routes_dir.exists() {
        return Ok(Vec::new());
    }

    let mut routes = Vec::new();
    let ignore_prefix = config.ignore_prefix();

    for entry in WalkDir::new(routes_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Only process .rs files
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }

        // Get the file name
        let file_name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(name) => name,
            None => continue,
        };

        // Skip ignored files
        if file_name.starts_with(ignore_prefix) {
            continue;
        }

        let relative = path
            .strip_prefix(routes_dir)
            .unwrap_or(path);

        let route_info = parse_route_file(file_name, relative, config);
        routes.push(route_info);
    }

    // Sort routes by specificity (more specific first)
    routes.sort_by(|a, b| {
        let a_static = a.route_pattern.matches('/').count();
        let b_static = b.route_pattern.matches('/').count();
        b_static.cmp(&a_static)
    });

    Ok(routes)
}

/// Parse a route file name into route information.
fn parse_route_file(file_name: &str, relative_path: &Path, config: &NaviConfig) -> RouteInfo {
    let is_root = file_name == "__root";
    let is_layout = file_name.starts_with('_') && !is_root;
    let is_index = file_name == config.index_token();
    let has_dynamic_segment = file_name.contains('$');

    let route_pattern = file_name_to_pattern(file_name, relative_path, config);
    let module_name = file_name.replace('-', "_").replace('.', "_");

    let parent = compute_parent(relative_path);

    RouteInfo {
        relative_path: relative_path.to_path_buf(),
        route_pattern,
        module_name,
        is_layout,
        is_index,
        is_root,
        has_dynamic_segment,
        parent,
    }
}

/// Convert a file name to a route pattern.
fn file_name_to_pattern(file_name: &str, relative_path: &Path, _config: &NaviConfig) -> String {
    let mut segments = Vec::new();

    // Process each path component
    for component in relative_path.parent().into_iter().flat_map(|p| p.iter()) {
        if let Some(comp) = component.to_str() {
            // Skip pathless groups (parenthesized)
            if comp.starts_with('(') && comp.ends_with(')') {
                continue;
            }
            // Skip ignored components
            if comp.starts_with('-') {
                continue;
            }
            segments.push(component_name_to_segment(comp));
        }
    }

    // Process the file name itself
    if file_name != "__root" && file_name != "index" && !file_name.starts_with('_') {
        segments.push(component_name_to_segment(file_name));
    }

    if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    }
}

/// Convert a component name to a route segment.
fn component_name_to_segment(name: &str) -> String {
    // Handle escaped segments [x]
    let escaped_re = Regex::new(r"^\[(.+)\]$").unwrap();
    if let Some(caps) = escaped_re.captures(name) {
        return caps[1].to_string();
    }

    // Handle optional parameters {-$param}
    let optional_re = Regex::new(r"^\{-\$(.+)\}$").unwrap();
    if let Some(caps) = optional_re.captures(name) {
        return format!("{{-${}}}", &caps[1]);
    }

    // Handle prefix/suffix patterns {$param}.ext
    let prefix_suffix_re = Regex::new(r"^\{\$(.+?)\}(.+)$").unwrap();
    if let Some(caps) = prefix_suffix_re.captures(name) {
        return format!("{{${}}}.{}", &caps[1], &caps[2]);
    }

    // Handle splat $
    if name == "$" {
        return "$".to_string();
    }

    // Handle dynamic segments $param
    if name.starts_with('$') {
        return format!("${}", &name[1..]);
    }

    name.to_string()
}

/// Compute the parent route for a given relative path.
fn compute_parent(relative_path: &Path) -> Option<String> {
    relative_path.parent().and_then(|p| {
        if p.as_os_str().is_empty() {
            None
        } else {
            p.to_str().map(|s| s.to_string())
        }
    })
}
