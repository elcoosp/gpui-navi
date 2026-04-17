// navi-codegen/src/scanner.rs
use crate::config::NaviConfig;
use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct RouteInfo {
    pub relative_path: PathBuf,
    pub route_pattern: String,
    pub module_name: String,
    pub route_type_name: String,
    pub route_id: String,
    pub is_layout: bool,
    pub is_index: bool,
    pub is_root: bool,
    pub has_dynamic_segment: bool,
    pub parent: Option<String>,
    pub cfg_feature: Option<String>,
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

        // Skip if starts with ignore prefix
        if file_name.starts_with(ignore_prefix) {
            continue;
        }

        let relative = path.strip_prefix(routes_dir).unwrap_or(path);

        // Check if the file actually contains a route definition
        let content = fs::read_to_string(path).unwrap_or_default();
        if !content.contains("define_route!") {
            // Skip files without route definitions (e.g., pure module files)
            continue;
        }

        let route_info = parse_route_file(file_name, relative, config, &content)?;
        routes.push(route_info);
    }

    // Deduplicate by route_id
    let mut seen = std::collections::HashSet::new();
    routes.retain(|r| seen.insert(r.route_id.clone()));

    // Ensure __root__ exists (if not found, create virtual)
    let has_root = routes.iter().any(|r| r.is_root);
    if !has_root {
        routes.push(RouteInfo {
            relative_path: PathBuf::from("__root.rs"),
            route_pattern: "/".to_string(),
            module_name: "__root".to_string(),
            route_type_name: "RootRoute".to_string(),
            route_id: "__root__".to_string(),
            is_layout: true,
            is_index: false,
            is_root: true,
            has_dynamic_segment: false,
            parent: None,
            cfg_feature: None,
        });
    }

    assign_parents(&mut routes, config);

    // Sort by depth (parents before children)
    routes.sort_by(|a, b| {
        let a_depth = a.relative_path.components().count();
        let b_depth = b.relative_path.components().count();
        a_depth.cmp(&b_depth)
    });

    Ok(routes)
}

fn parse_route_file(
    file_name: &str,
    relative_path: &Path,
    config: &NaviConfig,
    content: &str,
) -> Result<RouteInfo> {
    let is_root = file_name == "__root";
    let is_index = file_name == config.index_token()
        || (file_name == "mod"
            && relative_path
                .parent()
                .map(|p| p.file_name().unwrap_or_default() == config.index_token())
                .unwrap_or(false));
    let has_dynamic_segment = file_name.contains('$');

    let (effective_file_name, route_id_base) = if file_name == "mod" {
        let dir_name = relative_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("");
        (dir_name, dir_name.to_string())
    } else {
        (file_name, file_name.to_string())
    };

    let route_pattern = file_name_to_pattern(effective_file_name, relative_path);
    let module_name = build_module_path(relative_path, file_name == "mod");
    let route_type_name = extract_route_type_name(content, effective_file_name, relative_path);
    let route_id = route_type_name.clone(); // 👈 Node ID = route type name
    let is_layout = infer_layout(effective_file_name, relative_path);
    let cfg_feature = extract_cfg_feature(content);

    Ok(RouteInfo {
        relative_path: relative_path.to_path_buf(),
        route_pattern,
        module_name,
        route_type_name,
        route_id,
        is_layout,
        is_index,
        is_root,
        has_dynamic_segment,
        parent: None,
        cfg_feature,
    })
}
fn extract_cfg_feature(content: &str) -> Option<String> {
    let re = Regex::new(r#"#\[cfg\(feature\s*=\s*"([^"]+)"\)\]"#).unwrap();
    let define_re = Regex::new(r"define_route!").unwrap();

    let mut last_cfg: Option<String> = None;
    for line in content.lines() {
        let line = line.trim();
        if let Some(caps) = re.captures(line) {
            last_cfg = Some(caps[1].to_string());
        }
        if define_re.is_match(line) {
            return last_cfg;
        }
    }
    None
}

fn extract_route_type_name(content: &str, file_stem: &str, relative_path: &Path) -> String {
    let re = Regex::new(r"define_route!\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*[),]").unwrap();
    if let Some(caps) = re.captures(content) {
        return caps[1].to_string();
    }
    infer_route_type_name(file_stem, relative_path)
}

fn build_module_path(relative_path: &Path, is_mod_rs: bool) -> String {
    let mut components: Vec<String> = relative_path
        .parent()
        .into_iter()
        .flat_map(|p| p.iter())
        .map(|c| sanitize_module_ident(c.to_str().unwrap_or("")))
        .collect();

    if !is_mod_rs {
        // For regular files, add the file stem as the last component.
        let file_stem = relative_path.file_stem().unwrap().to_str().unwrap();
        components.push(sanitize_module_ident(file_stem));
    }
    // For mod.rs, the module is the parent directory itself, so no extra component.

    components.join("::")
}
fn sanitize_module_ident(name: &str) -> String {
    let name = name.replace('-', "_").replace('.', "_");
    if name.starts_with('$') {
        format!("param_{}", &name[1..])
    } else {
        match name.as_str() {
            "as" | "break" | "const" | "continue" | "crate" | "else" | "enum" | "extern"
            | "false" | "fn" | "for" | "if" | "impl" | "in" | "let" | "loop" | "match" | "mod"
            | "move" | "mut" | "pub" | "ref" | "return" | "self" | "Self" | "static" | "struct"
            | "super" | "trait" | "true" | "type" | "unsafe" | "use" | "where" | "while"
            | "async" | "await" | "dyn" | "union" => format!("r#{}", name),
            _ => name.to_string(),
        }
    }
}

fn file_name_to_pattern(file_name: &str, relative_path: &Path) -> String {
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

fn generate_route_id(file_name: &str, relative_path: &Path) -> String {
    let mut parts: Vec<String> = relative_path
        .parent()
        .into_iter()
        .flat_map(|p| p.iter())
        .map(|c| sanitize_module_ident(c.to_str().unwrap()))
        .collect();
    parts.push(sanitize_module_ident(file_name));
    parts.join("/")
}

fn infer_layout(file_name: &str, relative_path: &Path) -> bool {
    if file_name == "__root" {
        return true;
    }
    if file_name.starts_with('_') {
        return true;
    }
    if let Some(parent) = relative_path.parent() {
        let dir_name = file_name;
        let sibling_dir = parent.join(dir_name);
        sibling_dir.is_dir()
    } else {
        false
    }
}

fn infer_route_type_name(file_stem: &str, relative_path: &Path) -> String {
    let base = to_pascal_case(file_stem);
    if let Some(parent) = relative_path.parent() {
        if let Some(parent_name) = parent.file_name().and_then(|n| n.to_str()) {
            if parent_name != "routes" && !parent_name.is_empty() {
                let parent_pascal = to_pascal_case(parent_name);
                return format!("{}{}Route", parent_pascal, base);
            }
        }
    }
    format!("{}Route", base)
}

fn assign_parents(routes: &mut Vec<RouteInfo>, _config: &NaviConfig) {
    // Build a map from route_type_name to index
    let mut type_to_index: HashMap<String, usize> = HashMap::new();
    for (i, route) in routes.iter().enumerate() {
        type_to_index.insert(route.route_type_name.clone(), i);
    }

    for i in 0..routes.len() {
        let route = &routes[i];
        if route.is_root {
            continue;
        }

        let path = &route.relative_path;
        let parent_path = path.parent().unwrap_or(Path::new(""));

        if parent_path.as_os_str().is_empty() {
            // Top-level routes have parent RootRoute (type name)
            if let Some(&root_idx) = type_to_index.get("RootRoute") {
                routes[i].parent = Some(routes[root_idx].route_id.clone());
            }
            continue;
        }

        // Find the parent layout file in the parent directory
        let parent_dir_name = parent_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        // Possible parent file names: mod.rs, {dir_name}.rs, __root.rs
        let candidates = vec![
            parent_path.join("mod.rs"),
            parent_path.join(format!("{}.rs", parent_dir_name)),
            parent_path.join("__root.rs"),
        ];

        let mut found_parent = None;
        for candidate in candidates {
            if let Ok(content) = fs::read_to_string(&candidate) {
                if let Some(type_name) = extract_route_type_name_from_content(&content) {
                    if let Some(&parent_idx) = type_to_index.get(&type_name) {
                        found_parent = Some(routes[parent_idx].route_id.clone());
                        break;
                    }
                }
            }
        }

        // Fallback to RootRoute if no parent found
        if let Some(parent_id) = found_parent {
            routes[i].parent = Some(parent_id);
        } else if let Some(&root_idx) = type_to_index.get("RootRoute") {
            routes[i].parent = Some(routes[root_idx].route_id.clone());
        }
    }
}

// Helper to extract route type name from file content without full parsing
fn extract_route_type_name_from_content(content: &str) -> Option<String> {
    let re = Regex::new(r"define_route!\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*[),]").unwrap();
    re.captures(content).map(|caps| caps[1].to_string())
}
fn to_pascal_case(s: &str) -> String {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}
