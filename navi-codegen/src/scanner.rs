// navi-codegen/src/scanner.rs
use crate::config::NaviConfig;
use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct RouteInfo {
    pub relative_path: PathBuf,
    pub route_pattern: String,
    pub module_name: String,
    pub pascal_name: String,
    pub route_id: String,
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

    // Second pass: assign parents based on directory nesting
    assign_parents(&mut routes);

    // Sort by depth then specificity
    routes.sort_by(|a, b| {
        let a_depth = a.relative_path.components().count();
        let b_depth = b.relative_path.components().count();
        b_depth.cmp(&a_depth).then_with(|| {
            // Index routes after non-index at same depth
            a.is_index.cmp(&b.is_index)
        })
    });

    Ok(routes)
}

fn parse_route_file(file_name: &str, relative_path: &Path, config: &NaviConfig) -> RouteInfo {
    let is_root = file_name == "__root";
    let is_index = file_name == config.index_token();
    let has_dynamic_segment = file_name.contains('$');

    let route_pattern = file_name_to_pattern(file_name, relative_path);
    let module_name = build_module_path(relative_path);
    let pascal_name = to_pascal_case(&module_name);
    let route_id = generate_route_id(file_name, relative_path);
    let is_layout = infer_layout(file_name, relative_path);

    RouteInfo {
        relative_path: relative_path.to_path_buf(),
        route_pattern,
        module_name,
        pascal_name,
        route_id,
        is_layout,
        is_index,
        is_root,
        has_dynamic_segment,
        parent: None,
    }
}

/// Build a Rust module path from the relative file path, using sanitized identifiers.
fn build_module_path(relative_path: &Path) -> String {
    let mut components: Vec<String> = relative_path
        .parent()
        .into_iter()
        .flat_map(|p| p.iter())
        .map(|c| sanitize_module_ident(c.to_str().unwrap_or("")))
        .collect();

    let file_stem = relative_path.file_stem().unwrap().to_str().unwrap();
    components.push(sanitize_module_ident(file_stem));

    components.join("::")
}

/// Sanitize a name to a valid Rust identifier.
/// - Strips leading `$` and prefixes with `param_` if it becomes empty or conflicts.
/// - Replaces `-` and `.` with `_`.
fn sanitize_module_ident(name: &str) -> String {
    let mut name = name.replace('-', "_").replace('.', "_");
    if name.starts_with('$') {
        name = format!("param_{}", &name[1..]);
    }
    // Escape Rust keywords
    match name.as_str() {
        "as" | "break" | "const" | "continue" | "crate" | "else" | "enum" | "extern" | "false"
        | "fn" | "for" | "if" | "impl" | "in" | "let" | "loop" | "match" | "mod" | "move"
        | "mut" | "pub" | "ref" | "return" | "self" | "Self" | "static" | "struct" | "super"
        | "trait" | "true" | "type" | "unsafe" | "use" | "where" | "while" | "async" | "await"
        | "dyn" | "union" => format!("r#{}", name),
        _ => name,
    }
}

/// Convert a file name to a route pattern, preserving `$` segments.
fn file_name_to_pattern(file_name: &str, relative_path: &Path) -> String {
    let mut segments = Vec::new();

    for component in relative_path.parent().into_iter().flat_map(|p| p.iter()) {
        if let Some(comp) = component.to_str() {
            if comp.starts_with('(') && comp.ends_with(')') {
                continue; // pathless group
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

fn generate_route_id(file_name: &str, relative_path: &Path) -> String {
    let mut parts: Vec<String> = relative_path
        .parent()
        .into_iter()
        .flat_map(|p| p.iter())
        .map(|c| c.to_str().unwrap().to_string())
        .collect();
    parts.push(file_name.to_string());
    parts.join("/")
}

fn infer_layout(file_name: &str, relative_path: &Path) -> bool {
    if file_name == "__root" {
        return true;
    }
    if file_name.starts_with('_') {
        return true;
    }
    // A file is a layout if there is a directory with the same name alongside it.
    if let Some(parent) = relative_path.parent() {
        let dir_name = file_name;
        let sibling_dir = parent.join(dir_name);
        sibling_dir.is_dir()
    } else {
        false
    }
}

fn assign_parents(routes: &mut Vec<RouteInfo>) {
    // Map route id to index for quick lookup
    let mut id_to_index: HashMap<String, usize> = HashMap::new();
    for (i, route) in routes.iter().enumerate() {
        id_to_index.insert(route.route_id.clone(), i);
    }

    for i in 0..routes.len() {
        let route = &routes[i];
        let path = &route.relative_path;
        if let Some(parent_path) = path.parent() {
            // Find parent route: a file that is the immediate parent directory's index or layout
            if let Some(parent_dir) = parent_path.file_name().and_then(|n| n.to_str()) {
                // Try to find a layout file with the same name as the directory
                let parent_file = parent_path.join("mod").with_extension("rs");
                let parent_layout_file = parent_path
                    .join(format!("_{}", parent_dir))
                    .with_extension("rs");
                let parent_index_file = parent_path.join("index").with_extension("rs");

                let candidate_files = [
                    parent_path.join(format!("{}.rs", parent_dir)),
                    parent_layout_file,
                    parent_index_file,
                ];

                for candidate in candidate_files {
                    if let Some(candidate_stem) = candidate.file_stem().and_then(|s| s.to_str()) {
                        let candidate_id = generate_route_id(candidate_stem, &candidate);
                        if let Some(&parent_idx) = id_to_index.get(&candidate_id) {
                            routes[i].parent = Some(routes[parent_idx].route_id.clone());
                            break;
                        }
                    }
                }
            }
        }
    }
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
