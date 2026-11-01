use crate::config::NaviConfig;
use anyhow::Result;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use walkdir::WalkDir;

/// Classification of a discovered route file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteKind {
    /// The explicit or synthetic `__root` layout.
    Root,
    /// A layout route: `_pathless.rs`, a `dir.rs` beside a `dir/` directory,
    /// or a file declaring `is_layout: true`.
    Layout,
    /// An `index` route.
    Index,
    /// A `$` catch-all not-found route.
    NotFound,
    /// A regular page route.
    Page,
}

#[derive(Debug, Clone)]
pub struct RouteInfo {
    pub relative_path: PathBuf,
    pub route_pattern: String,
    pub module_name: String,
    pub route_type_name: String,
    pub route_id: String,
    pub kind: RouteKind,
    pub has_dynamic_segment: bool,
    pub parent: Option<String>,
    pub cfg_feature: Option<String>,
}

impl RouteInfo {
    /// Whether this route acts as a layout for its children.
    #[must_use]
    pub fn is_layout(&self) -> bool {
        matches!(self.kind, RouteKind::Root | RouteKind::Layout)
    }

    /// Whether this is an index route.
    #[must_use]
    pub fn is_index(&self) -> bool {
        matches!(self.kind, RouteKind::Index)
    }

    /// Whether this is the tree root.
    #[must_use]
    pub fn is_root(&self) -> bool {
        matches!(self.kind, RouteKind::Root)
    }

    /// Whether this is a `$` not-found route.
    #[must_use]
    pub fn is_not_found(&self) -> bool {
        matches!(self.kind, RouteKind::NotFound)
    }
}

struct ScannerRegexes {
    route_type: Regex,
    cfg: Regex,
    define: Regex,
    is_layout_true: Regex,
}

fn regexes() -> &'static ScannerRegexes {
    static RE: OnceLock<ScannerRegexes> = OnceLock::new();
    RE.get_or_init(|| ScannerRegexes {
        route_type: Regex::new(r"define_route!\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*[,)]")
            .expect("valid route-type regex"),
        cfg: Regex::new(r#"#\[cfg\(feature\s*=\s*"([^"]+)"\)\]"#).expect("valid cfg regex"),
        define: Regex::new(r"define_route!").expect("valid define regex"),
        is_layout_true: Regex::new(r"is_layout\s*:\s*true").expect("valid is_layout regex"),
    })
}

/// Walk the routes directory and derive a [`RouteInfo`] for every file that
/// declares a route via `define_route!`.
///
/// # Errors
/// Returns an error when a route file contains `define_route!` but no
/// parsable route type name.
pub fn scan_routes(config: &NaviConfig) -> Result<Vec<RouteInfo>> {
    let routes_dir = Path::new(&config.routes_directory);
    if !routes_dir.exists() {
        return Ok(Vec::new());
    }

    let mut routes = Vec::new();
    let ignore_prefix = config.ignore_prefix();
    let res = regexes();

    for entry in WalkDir::new(routes_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }

        let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };

        if file_name.starts_with(ignore_prefix) {
            continue;
        }

        let relative = path.strip_prefix(routes_dir).unwrap_or(path);
        let content = fs::read_to_string(path).unwrap_or_default();
        if !res.define.is_match(&content) {
            continue;
        }

        let route_info = parse_route_file(file_name, relative, config, &content, res)?;
        routes.push(route_info);
    }

    let mut seen = std::collections::HashSet::new();
    routes.retain(|r| seen.insert(r.route_id.clone()));

    if !routes.iter().any(RouteInfo::is_root) {
        routes.push(RouteInfo {
            relative_path: PathBuf::from("__root.rs"),
            route_pattern: "/".to_string(),
            module_name: "__root".to_string(),
            route_type_name: "RootRoute".to_string(),
            route_id: "__root__".to_string(),
            kind: RouteKind::Root,
            has_dynamic_segment: false,
            parent: None,
            cfg_feature: None,
        });
    }

    assign_parents(&mut routes);

    // Deterministic ordering: shallower routes first, then by path.
    routes.sort_by(|a, b| {
        a.relative_path
            .components()
            .count()
            .cmp(&b.relative_path.components().count())
            .then_with(|| a.relative_path.cmp(&b.relative_path))
    });

    warn_duplicate_patterns(&routes);

    Ok(routes)
}

fn warn_duplicate_patterns(routes: &[RouteInfo]) {
    let mut pattern_map: std::collections::BTreeMap<String, Vec<&str>> =
        std::collections::BTreeMap::new();
    for route in routes {
        pattern_map
            .entry(route.route_pattern.clone())
            .or_default()
            .push(&route.route_id);
    }
    for (pattern, ids) in pattern_map {
        if ids.len() > 1 {
            eprintln!(
                "Warning: Duplicate route pattern '{pattern}' detected (used by {ids:?}). \
                 Ensure layout/index are properly nested."
            );
        }
    }
}

fn parse_route_file(
    file_name: &str,
    relative_path: &Path,
    config: &NaviConfig,
    content: &str,
    res: &ScannerRegexes,
) -> Result<RouteInfo> {
    let is_root = file_name == "__root";
    let is_index = file_name == config.index_token()
        || (file_name == "mod"
            && relative_path
                .parent()
                .is_some_and(|p| p.file_name().unwrap_or_default() == config.index_token()));
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
                .is_some_and(|p| p.file_name().unwrap_or_default() == "$"));

    // For `mod.rs` the route *is* its directory; the parent path already
    // contributed the segment, so no extra file segment is appended.
    let pattern_file_name = if file_name == "mod" {
        None
    } else {
        Some(effective_file_name)
    };
    let route_pattern = file_name_to_pattern(pattern_file_name, relative_path);
    let module_name = build_module_path(relative_path, file_name == "mod");
    let route_type_name = extract_route_type_name(content, relative_path, res)?;
    let route_id = route_type_name.clone();
    let is_layout = is_root || infer_layout(effective_file_name, relative_path, content, config);
    let kind = if is_root {
        RouteKind::Root
    } else if is_layout {
        RouteKind::Layout
    } else if is_not_found {
        RouteKind::NotFound
    } else if is_index {
        RouteKind::Index
    } else {
        RouteKind::Page
    };
    let cfg_feature = extract_cfg_feature(content, res);

    Ok(RouteInfo {
        relative_path: relative_path.to_path_buf(),
        route_pattern,
        module_name,
        route_type_name,
        route_id,
        kind,
        has_dynamic_segment,
        parent: None,
        cfg_feature,
    })
}

fn extract_route_type_name(
    content: &str,
    relative_path: &Path,
    res: &ScannerRegexes,
) -> Result<String> {
    res.route_type
        .captures(content)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .ok_or_else(|| anyhow::anyhow!("No define_route! found in {}", relative_path.display()))
}

fn extract_cfg_feature(content: &str, res: &ScannerRegexes) -> Option<String> {
    let mut last_cfg: Option<String> = None;
    for line in content.lines() {
        let line = line.trim();
        if let Some(caps) = res.cfg.captures(line) {
            last_cfg = Some(caps[1].to_string());
        }
        if res.define.is_match(line) {
            return last_cfg;
        }
    }
    None
}

fn file_stem_to_module_ident(stem: &str) -> String {
    // Route groups like `(marketing)` are transparent for module names too.
    let stripped = stem
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(stem);
    let s = stripped.replace(['-', '.'], "_");
    let ident = if s == "$" {
        "splat".to_string()
    } else if let Some(rest) = s.strip_prefix('$') {
        format!("param_{rest}")
    } else {
        s
    };
    escape_rust_keyword(ident)
}

fn escape_rust_keyword(s: String) -> String {
    const KEYWORDS: [&str; 46] = [
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while", "async", "await", "dyn", "union", "gen", "box", "priv",
        "abstract", "become", "do", "final",
    ];
    if KEYWORDS.contains(&s.as_str()) {
        format!("r#{s}")
    } else {
        s
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
        let file_stem = relative_path
            .file_stem()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();
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

fn file_name_to_pattern(file_name: Option<&str>, relative_path: &Path) -> String {
    let mut segments = Vec::new();
    for component in relative_path.parent().into_iter().flat_map(|p| p.iter()) {
        if let Some(comp) = component.to_str() {
            if comp.starts_with('(') || comp.starts_with('_') || comp.starts_with('-') {
                continue;
            }
            if let Some(seg) = file_stem_to_url_segment(comp) {
                segments.push(seg);
            }
        }
    }
    if let Some(seg) = file_name.and_then(file_stem_to_url_segment) {
        segments.push(seg);
    }
    if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    }
}

fn infer_layout(file_name: &str, relative_path: &Path, content: &str, config: &NaviConfig) -> bool {
    if file_name == "__root" || file_name.starts_with('_') {
        return true;
    }
    // Route files can declare `is_layout: true` explicitly in `define_route!`;
    // honour that so generated parents match the runtime tree.
    if regexes().is_layout_true.is_match(content) {
        return true;
    }
    // A `users.rs` next to a `users/` directory is that directory's layout.
    // The check must be resolved against the configured routes directory,
    // not the process working directory.
    relative_path
        .parent()
        .map(|parent| {
            Path::new(&config.routes_directory)
                .join(parent)
                .join(file_name)
        })
        .is_some_and(|sibling| sibling.is_dir())
}

fn assign_parents(routes: &mut [RouteInfo]) {
    let mut dir_to_layout: std::collections::HashMap<PathBuf, String> =
        std::collections::HashMap::new();

    for route in routes.iter() {
        if route.is_layout() {
            let dir = if route.relative_path.file_name().unwrap_or_default() == "mod.rs" {
                route
                    .relative_path
                    .parent()
                    .unwrap_or(Path::new(""))
                    .to_path_buf()
            } else {
                route.relative_path.with_extension("")
            };
            dir_to_layout.insert(dir, route.route_id.clone());
        }
    }

    let root_id = routes
        .iter()
        .find(|r| r.is_root())
        .map(|r| r.route_id.clone());

    for route in routes.iter_mut() {
        if route.is_root() {
            continue;
        }
        let search = route
            .relative_path
            .parent()
            .unwrap_or(Path::new(""))
            .to_path_buf();
        route.parent = match dir_to_layout.get(&search) {
            Some(layout_id) if layout_id != &route.route_id => Some(layout_id.clone()),
            _ => root_id.clone(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Create a temp route tree fixture and a config pointing at it.
    fn fixture(files: &[(&str, &str)]) -> (tempdir::TempDir, NaviConfig) {
        let dir = tempdir::tempdir();
        let routes_dir = dir.path().join("src/routes");
        std::fs::create_dir_all(&routes_dir).expect("mkdir routes");
        for (rel, content) in files {
            let path = routes_dir.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("mkdir");
            }
            fs::write(path, content).expect("write");
        }
        let config = NaviConfig {
            routes_directory: routes_dir.to_string_lossy().into_owned(),
            generated_route_tree: dir
                .path()
                .join("src/route_tree.gen.rs")
                .to_string_lossy()
                .into_owned(),
            ..NaviConfig::default()
        };
        (dir, config)
    }

    const DEFINE: &str = "define_route!(TestRoute, path: \"/\", component: T);";

    #[test]
    fn test_missing_dir_yields_empty() {
        let config = NaviConfig {
            routes_directory: "/definitely/not/here".into(),
            ..NaviConfig::default()
        };
        let routes = scan_routes(&config).expect("ok");
        assert!(routes.is_empty(), "missing dir yields no routes at all");
    }

    #[test]
    fn test_empty_dir_yields_synthetic_root() {
        let (_d, config) = fixture(&[]);
        let routes = scan_routes(&config).expect("ok");
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].route_id, "__root__");
        assert!(routes[0].is_root());
        assert!(routes[0].is_layout());
        assert_eq!(routes[0].route_pattern, "/");
    }

    #[test]
    fn test_file_without_define_route_is_skipped() {
        let (_d, config) = fixture(&[("helpers.rs", "pub fn helper() {}")]);
        let routes = scan_routes(&config).expect("ok");
        assert_eq!(routes.len(), 1, "only synthetic root");
    }

    #[test]
    fn test_basic_static_route() {
        let (_d, config) = fixture(&[("about.rs", DEFINE)]);
        let routes = scan_routes(&config).expect("ok");
        let about = routes
            .iter()
            .find(|r| r.route_id == "TestRoute")
            .expect("found");
        assert_eq!(about.route_pattern, "/about");
        assert_eq!(about.module_name, "about");
        assert!(!about.is_layout());
        assert!(!about.is_index());
        assert_eq!(about.parent.as_deref(), Some("__root__"));
    }

    #[test]
    fn test_root_route_detected() {
        let (_d, config) = fixture(&[("__root.rs", DEFINE)]);
        let routes = scan_routes(&config).expect("ok");
        assert_eq!(routes.len(), 1, "no synthetic root needed");
        assert!(routes[0].is_root());
        assert!(routes[0].is_layout());
    }

    #[test]
    fn test_index_route() {
        let (_d, config) = fixture(&[("index.rs", DEFINE)]);
        let routes = scan_routes(&config).expect("ok");
        let idx = routes
            .iter()
            .find(|r| r.route_id == "TestRoute")
            .expect("found");
        assert!(idx.is_index());
        assert_eq!(idx.route_pattern, "/");
    }

    #[test]
    fn test_dynamic_route_pattern_and_flag() {
        let (_d, config) = fixture(&[("users/$id.rs", DEFINE)]);
        let routes = scan_routes(&config).expect("ok");
        let r = routes
            .iter()
            .find(|r| r.route_id == "TestRoute")
            .expect("found");
        assert_eq!(r.route_pattern, "/users/$id");
        assert!(r.has_dynamic_segment);
        assert_eq!(r.module_name, "users::param_id");
    }

    #[test]
    fn test_splat_module_ident() {
        let (_d, config) = fixture(&[("docs/$.rs", DEFINE)]);
        let routes = scan_routes(&config).expect("ok");
        let r = routes
            .iter()
            .find(|r| r.route_id == "TestRoute")
            .expect("found");
        assert_eq!(r.route_pattern, "/docs/$");
        assert_eq!(r.module_name, "docs::splat");
        assert!(r.is_not_found());
    }

    #[test]
    fn test_layout_route_for_dir_with_siblings() {
        let (_d, config) = fixture(&[
            ("users.rs", "define_route!(UsersLayout, path: \"/users\");"),
            (
                "users/index.rs",
                "define_route!(UsersIndex, path: \"/users\");",
            ),
        ]);
        let routes = scan_routes(&config).expect("ok");
        let layout = routes
            .iter()
            .find(|r| r.route_id == "UsersLayout")
            .expect("found");
        assert!(layout.is_layout(), "users.rs has a users/ sibling dir");
        let index = routes
            .iter()
            .find(|r| r.route_id == "UsersIndex")
            .expect("found");
        assert!(index.is_index());
        assert_eq!(index.parent.as_deref(), Some("UsersLayout"));
    }

    #[test]
    fn test_pathless_layout_underscore_is_layout_and_not_in_pattern() {
        let (_d, config) = fixture(&[("_auth/profile.rs", DEFINE)]);
        let routes = scan_routes(&config).expect("ok");
        let profile = routes
            .iter()
            .find(|r| r.route_id == "TestRoute")
            .expect("found");
        assert_eq!(profile.route_pattern, "/profile");
        // `_auth` dir itself produces no route.
        assert_eq!(routes.len(), 2, "profile + synthetic root");
    }

    #[test]
    fn test_route_groups_parentheses_are_transparent() {
        let (_d, config) = fixture(&[("(marketing)/about.rs", DEFINE)]);
        let routes = scan_routes(&config).expect("ok");
        let about = routes
            .iter()
            .find(|r| r.route_id == "TestRoute")
            .expect("found");
        assert_eq!(about.route_pattern, "/about");
        assert_eq!(about.module_name, "marketing::about");
    }

    #[test]
    fn test_ignore_prefix_skips_files() {
        let (_d, config) = fixture(&[("-draft.rs", DEFINE)]);
        let routes = scan_routes(&config).expect("ok");
        assert_eq!(routes.len(), 1, "only synthetic root");
    }

    #[test]
    fn test_cfg_feature_extracted() {
        let content = "#[cfg(feature = \"admin\")]\ndefine_route!(AdminRoute, path: \"/admin\");";
        let (_d, config) = fixture(&[("admin.rs", content)]);
        let routes = scan_routes(&config).expect("ok");
        let admin = routes
            .iter()
            .find(|r| r.route_id == "AdminRoute")
            .expect("found");
        assert_eq!(admin.cfg_feature.as_deref(), Some("admin"));
    }

    #[test]
    fn test_cfg_before_unrelated_attr_not_stolen() {
        let content = "use std::fmt;\n// some comment\ndefine_route!(R, path: \"/x\");";
        let (_d, config) = fixture(&[("x.rs", content)]);
        let routes = scan_routes(&config).expect("ok");
        let r = routes.iter().find(|r| r.route_id == "R").expect("found");
        assert_eq!(r.cfg_feature, None);
    }

    #[test]
    fn test_duplicate_ids_deduped() {
        let (_d, config) = fixture(&[
            ("a.rs", "define_route!(Same, path: \"/a\");"),
            ("b.rs", "define_route!(Same, path: \"/b\");"),
        ]);
        let routes = scan_routes(&config).expect("ok");
        let same = routes.iter().filter(|r| r.route_id == "Same").count();
        assert_eq!(same, 1, "duplicate ids are deduped");
    }

    #[test]
    fn test_mod_rs_inside_named_dir_is_dir_route() {
        let (_d, config) = fixture(&[
            (
                "users/mod.rs",
                "define_route!(UsersDir, path: \"/users\", is_layout: true);",
            ),
            (
                "users/$id.rs",
                "define_route!(UserId, path: \"/users/$id\");",
            ),
        ]);
        let routes = scan_routes(&config).expect("ok");
        let dir_route = routes
            .iter()
            .find(|r| r.route_id == "UsersDir")
            .expect("found");
        assert!(
            dir_route.is_layout(),
            "explicit is_layout: true must be honored"
        );
        assert_eq!(dir_route.route_pattern, "/users");
        assert_eq!(dir_route.module_name, "users");
        let child = routes
            .iter()
            .find(|r| r.route_id == "UserId")
            .expect("found");
        assert_eq!(child.parent.as_deref(), Some("UsersDir"));
    }

    #[test]
    fn test_rust_keyword_file_escaped() {
        let (_d, config) = fixture(&[("type.rs", DEFINE)]);
        let routes = scan_routes(&config).expect("ok");
        let r = routes
            .iter()
            .find(|r| r.route_id == "TestRoute")
            .expect("found");
        assert_eq!(r.module_name, "r#type");
    }

    #[test]
    fn test_dashed_file_name() {
        let (_d, config) = fixture(&[("my-page.rs", DEFINE)]);
        let routes = scan_routes(&config).expect("ok");
        let r = routes
            .iter()
            .find(|r| r.route_id == "TestRoute")
            .expect("found");
        assert_eq!(r.route_pattern, "/my-page");
        assert_eq!(r.module_name, "my_page");
    }

    #[test]
    fn test_no_define_route_errors() {
        let (_d, config) = fixture(&[("broken.rs", "define_route!(Broken")]);
        // Has the "!" but no route type name follows.
        let result = scan_routes(&config);
        assert!(result.is_err(), "missing type name must error");
    }

    #[test]
    fn test_custom_index_token() {
        let (d, mut config) = fixture(&[("home.rs", DEFINE)]);
        config.index_token = Some("home".into());
        let routes = scan_routes(&config).expect("ok");
        let r = routes
            .iter()
            .find(|r| r.route_id == "TestRoute")
            .expect("found");
        assert!(r.is_index());
        assert_eq!(
            r.route_pattern, "/home",
            "custom index token keeps its segment"
        );
        drop(d);
    }

    #[test]
    fn test_sorting_is_depth_then_path() {
        let (_d, config) = fixture(&[
            ("z.rs", "define_route!(Z, path: \"/z\");"),
            ("a/b/c.rs", "define_route!(C, path: \"/a/b/c\");"),
            ("a.rs", "define_route!(A, path: \"/a\");"),
        ]);
        let routes = scan_routes(&config).expect("ok");
        let ids: Vec<&str> = routes.iter().map(|r| r.route_id.as_str()).collect();
        assert_eq!(ids, vec!["__root__", "A", "Z", "C"]);
    }
}

/// Minimal temp-dir helper so the scanner tests can build on-disk fixtures
/// without pulling in an external crate.
#[cfg(test)]
mod tempdir {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    pub struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        pub fn path(&self) -> &std::path::Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    pub fn tempdir() -> TempDir {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path =
            std::env::temp_dir().join(format!("navi-codegen-test-{}-{}", std::process::id(), n));
        std::fs::create_dir_all(&path).expect("create temp dir");
        TempDir { path }
    }
}
