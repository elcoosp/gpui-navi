use crate::config::NaviConfig;
use anyhow::Result;
use regex::Regex;
use std::collections::{HashMap, HashSet};
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
    pub is_not_found: bool,
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

        if file_name.starts_with(ignore_prefix) {
            continue;
        }

        let relative = path.strip_prefix(routes_dir).unwrap_or(path);
        let content = fs::read_to_string(path).unwrap_or_default();
        if !content.contains("define_route!") {
            continue;
        }

        let route_info = parse_route_file(file_name, relative, config, &content)?;
        routes.push(route_info);
    }

    let mut seen = HashSet::new();
    routes.retain(|r| seen.insert(r.route_id.clone()));

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
            is_not_found: false,
        });
    }

    assign_parents(&mut routes);

    routes.sort_by(|a, b| {
        let a_depth = a.relative_path.components().count();
        let b_depth = b.relative_path.components().count();
        a_depth.cmp(&b_depth)
    });

    // Check for duplicate route patterns (allow layout + index pairs)
    let mut pattern_map: HashMap<String, Vec<&RouteInfo>> = HashMap::new();
    for route in &routes {
        pattern_map
            .entry(route.route_pattern.clone())
            .or_default()
            .push(route);
    }
    for (pattern, routes_with_pattern) in pattern_map {
            // Allow if routes have different cfg features (they won't be compiled together)
            let mut features: HashSet<Option<String>> = HashSet::new();
            for r in &routes_with_pattern {
                features.insert(r.cfg_feature.clone());
            }
            if features.len() == routes_with_pattern.len() {
                continue; // all have distinct cfg features, allowed
            }
            
            // Otherwise, allow only if one is a layout and the other is an index route that is a child of that layout
    }

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

    let effective_file_name = if file_name == "mod" {
        relative_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("")
    } else {
        file_name
    };

    let is_not_found = file_name == "$"
        || (file_name == "mod"
            && relative_path
                .parent()
                .map(|p| p.file_name().unwrap_or_default() == "$")
                .unwrap_or(false));

    let route_pattern = file_name_to_pattern(effective_file_name, relative_path);
    let module_name = build_module_path(relative_path, file_name == "mod");
    let route_type_name = extract_route_type_name(content, relative_path)?;
    let route_id = route_type_name.clone();
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
        is_not_found,
    })
}

fn extract_route_type_name(content: &str, relative_path: &Path) -> Result<String> {
    let re = Regex::new(r"define_route!\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*[,)]").unwrap();
    re.captures(content)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .ok_or_else(|| anyhow::anyhow!("No define_route! found in {:?}", relative_path))
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

fn file_stem_to_module_ident(stem: &str) -> String {
    let s = stem.replace(['-', '.'], "_");
    let ident = if s == "$" {
        "splat".to_string()
    } else if s.starts_with('$') {
        format!("param_{}", &s[1..])
    } else {
        s
    };
    escape_rust_keyword(ident)
}

fn escape_rust_keyword(s: String) -> String {
    match s.as_str() {
        "as" | "break" | "const" | "continue" | "crate" | "else" | "enum" | "extern"
        | "false" | "fn" | "for" | "if" | "impl" | "in" | "let" | "loop" | "match"
        | "mod" | "move" | "mut" | "pub" | "ref" | "return" | "self" | "Self"
        | "static" | "struct" | "super" | "trait" | "true" | "type" | "unsafe"
        | "use" | "where" | "while" | "async" | "await" | "dyn" | "union"
        => format!("r#{}", s),
        _ => s,
    }
}

fn build_module_path(relative_path: &Path, is_mod_rs: bool) -> String {
    let mut components: Vec<String> = relative_path
        .parent()
        .into_iter()
        .flat_map(|p| p.iter())
        .map(|c| file_stem_to_module_ident(c.to_str().unwrap_or("")))
        .collect();

    if !is_mod_rs {
        let file_stem = relative_path.file_stem().unwrap().to_str().unwrap();
        components.push(file_stem_to_module_ident(file_stem));
    }

    components.join("::")
}

fn file_stem_to_url_segment(name: &str) -> Option<String> {
    if matches!(name, "__root" | "index") {
        return None;
    }
    if name.starts_with('_') {
        return None;
    }
    if name.starts_with('(') && name.ends_with(')') {
        return None;
    }
    Some(name.to_string())
}

fn file_name_to_pattern(file_name: &str, relative_path: &Path) -> String {
    let mut segments = Vec::new();
    for component in relative_path.parent().into_iter().flat_map(|p| p.iter()) {
        if let Some(comp) = component.to_str() {
            if comp.starts_with('(') && comp.ends_with(')') {
                continue;
            }
            if comp.starts_with('_') {
                continue;
            }
            if comp.starts_with('-') {
                continue;
            }
            if let Some(seg) = file_stem_to_url_segment(comp) {
                segments.push(seg);
            }
        }
    }
    if let Some(seg) = file_stem_to_url_segment(file_name) {
        segments.push(seg);
    }
    if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    }
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

fn assign_parents(routes: &mut Vec<RouteInfo>) {
    use std::collections::HashMap;
    use std::path::PathBuf;

    let mut dir_to_layout: HashMap<PathBuf, String> = HashMap::new();

    for route in routes.iter() {
        if route.is_layout || route.is_root {
            let dir = if route.relative_path.file_name().unwrap() == "mod.rs" {
                route.relative_path.parent().unwrap().to_path_buf()
            } else {
                route.relative_path.with_extension("")
            };
            dir_to_layout.insert(dir, route.route_id.clone());
        }
    }

    let root_id = routes
        .iter()
        .find(|r| r.is_root)
        .map(|r| r.route_id.clone());

    for route in routes.iter_mut() {
        if route.is_root {
            continue;
        }
        let search = route
            .relative_path
            .parent()
            .unwrap_or(Path::new(""))
            .to_path_buf();
        if let Some(layout_id) = dir_to_layout.get(&search) {
            route.parent = Some(layout_id.clone());
        } else {
            route.parent = root_id.clone();
        }
    }
}
