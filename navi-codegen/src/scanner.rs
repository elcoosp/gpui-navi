use crate::config::NaviConfig;
use anyhow::Result;
use regex::Regex;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct RouteInfo {
    pub relative_path: PathBuf,
    pub route_pattern: String,
    pub module_name: String,
    pub pascal_name: String,
    pub is_layout: bool,
    pub is_index: bool,
    pub is_root: bool,
    pub has_dynamic_segment: bool,
    pub parent: Option<String>,
}

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

        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }

        let file_name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(name) => name,
            None => continue,
        };

        // Skip mod.rs files – they are module roots, not routes
        if file_name == "mod" {
            continue;
        }

        if file_name.starts_with(ignore_prefix) {
            continue;
        }

        let relative = path.strip_prefix(routes_dir).unwrap_or(path);
        let route_info = parse_route_file(file_name, relative, config);
        routes.push(route_info);
    }

    routes.sort_by(|a, b| {
        let a_static = a.route_pattern.matches('/').count();
        let b_static = b.route_pattern.matches('/').count();
        b_static.cmp(&a_static)
    });

    Ok(routes)
}

fn parse_route_file(file_name: &str, relative_path: &Path, config: &NaviConfig) -> RouteInfo {
    let is_root = file_name == "__root";
    let is_layout = file_name.starts_with('_') && !is_root;
    let is_index = file_name == config.index_token();
    let has_dynamic_segment = file_name.contains('$');

    let route_pattern = file_name_to_pattern(file_name, relative_path, config);
    let module_name = build_module_path(relative_path);
    let pascal_name = to_pascal_case(&module_name);

    let parent = compute_parent(relative_path);

    RouteInfo {
        relative_path: relative_path.to_path_buf(),
        route_pattern,
        module_name,
        pascal_name,
        is_layout,
        is_index,
        is_root,
        has_dynamic_segment,
        parent,
    }
}

/// Build a Rust module path from the relative file path.
/// Example: "users/_dollar_id.rs" -> "users::_dollar_id"
fn build_module_path(relative_path: &Path) -> String {
    let mut components: Vec<String> = relative_path
        .parent()
        .into_iter()
        .flat_map(|p| p.iter())
        .map(|c| sanitize_module_name(c.to_str().unwrap_or("")))
        .collect();

    let file_stem = relative_path.file_stem().unwrap().to_str().unwrap();
    components.push(sanitize_module_name(file_stem));

    components.join("::")
}

/// Sanitize a name to a valid Rust identifier, escaping keywords.
fn sanitize_module_name(name: &str) -> String {
    let name = name
        .replace('-', "_")
        .replace('.', "_")
        .replace('$', "_dollar_");
    match name.as_str() {
        // Rust keywords
        "as" | "break" | "const" | "continue" | "crate" | "else" | "enum" | "extern" | "false"
        | "fn" | "for" | "if" | "impl" | "in" | "let" | "loop" | "match" | "mod" | "move"
        | "mut" | "pub" | "ref" | "return" | "self" | "Self" | "static" | "struct" | "super"
        | "trait" | "true" | "type" | "unsafe" | "use" | "where" | "while" | "async" | "await"
        | "dyn" | "union" => format!("r#{}", name),
        _ => name,
    }
}

/// Convert a file name to a route pattern.
fn file_name_to_pattern(file_name: &str, relative_path: &Path, _config: &NaviConfig) -> String {
    let mut segments = Vec::new();

    for component in relative_path.parent().into_iter().flat_map(|p| p.iter()) {
        if let Some(comp) = component.to_str() {
            if comp.starts_with('(') && comp.ends_with(')') {
                continue;
            }
            if comp.starts_with('-') {
                continue;
            }
            segments.push(component_name_to_segment(comp));
        }
    }

    if file_name != "__root" && file_name != "index" && !file_name.starts_with('_') {
        segments.push(component_name_to_segment(file_name));
    }

    if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    }
}

fn component_name_to_segment(name: &str) -> String {
    let escaped_re = Regex::new(r"^\[(.+)\]$").unwrap();
    if let Some(caps) = escaped_re.captures(name) {
        return caps[1].to_string();
    }

    let optional_re = Regex::new(r"^\{-\$(.+)\}$").unwrap();
    if let Some(caps) = optional_re.captures(name) {
        return format!("{{-${}}}", &caps[1]);
    }

    let prefix_suffix_re = Regex::new(r"^\{\$(.+?)\}(.+)$").unwrap();
    if let Some(caps) = prefix_suffix_re.captures(name) {
        return format!("{{${}}}.{}", &caps[1], &caps[2]);
    }

    if name == "$" {
        return "$".to_string();
    }

    if name.starts_with('$') {
        return format!("${}", &name[1..]);
    }

    name.to_string()
}

fn compute_parent(relative_path: &Path) -> Option<String> {
    relative_path.parent().and_then(|p| {
        if p.as_os_str().is_empty() {
            None
        } else {
            Some(build_module_path(p))
        }
    })
}

fn to_pascal_case(name: &str) -> String {
    name.split("::")
        .flat_map(|s| s.split('_'))
        .flat_map(|s| s.split('-'))
        .filter(|s| !s.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}
