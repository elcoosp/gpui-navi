//! TanStack-style router link rendering a `quoin_ui::Node`.

use crate::state::RouterHandle;
use crate::{NavigateOptions, Navigator};
use quoin_ui::Node;
use quoin_ui::node::{NodeChild, NodeStyle, StyleCmd};
use std::collections::HashSet;
use std::sync::LazyLock;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PreloadType {
    /// Preload when the link enters the viewport. Adapters have no
    /// intersection observer yet, so this behaves like
    /// [`PreloadType::Render`].
    Viewport,
    /// Preload on hover (first mouse move over the link), after the delay.
    Intent,
    /// Preload as soon as the link renders.
    #[default]
    Render,
}

#[derive(Clone, Default)]
pub struct ActiveOptions {
    /// Require the full pathname to match (no segment-boundary prefix).
    pub exact: bool,
    /// Also compare the hash when deciding active state.
    pub include_hash: bool,
    /// Also compare the search params when deciding active state.
    pub include_search: bool,
}

/// Tracks which hrefs already kicked off a preload so renders and repeated
/// mouse moves do not spam the query cache.
static PRELOAD_STARTED: LazyLock<std::sync::Mutex<HashSet<String>>> =
    LazyLock::new(|| std::sync::Mutex::new(HashSet::new()));

fn mark_preload_started(href: &str) -> bool {
    PRELOAD_STARTED
        .lock()
        .expect("preload registry lock")
        .insert(href.to_string())
}

/// TanStack-style segment-boundary prefix match: `/users` is active for
/// `/users` and `/users/42` but not for `/users-admin`.
#[must_use]
pub fn path_is_active(current: &str, href: &str, exact: bool) -> bool {
    if exact {
        return current == href;
    }
    if href == "/" {
        return current.starts_with('/');
    }
    current == href || current.starts_with(&format!("{href}/"))
}

/// Whether the href points outside the router (absolute web URL or a
/// non-http scheme such as `mailto:`). External links are handed to the OS
/// instead of being matched against the route tree.
#[must_use]
pub fn is_external_href(href: &str) -> bool {
    href.starts_with("http://")
        || href.starts_with("https://")
        || href.starts_with("mailto:")
        || href.starts_with("tel:")
}

/// A framework-agnostic `<Link>`: builds a pointer-styled container with
/// navigation, active styling and preload behavior.
pub struct Link {
    href: String,
    search: Option<serde_json::Value>,
    hash: Option<String>,
    state: Option<serde_json::Value>,
    replace: bool,
    preload: Option<PreloadType>,
    preload_delay: Option<Duration>,
    disabled: bool,
    active_options: ActiveOptions,
    reset_scroll: bool,
    active_style: Option<Box<dyn Fn(NodeStyle) -> NodeStyle>>,
    inactive_style: Option<Box<dyn Fn(NodeStyle) -> NodeStyle>>,
    children: Vec<quoin_ui::Node>,
}

impl Link {
    #[must_use]
    pub fn new(to: impl Into<String>) -> Self {
        Self {
            href: to.into(),
            search: None,
            hash: None,
            state: None,
            replace: false,
            preload: None,
            preload_delay: None,
            disabled: false,
            active_options: ActiveOptions::default(),
            reset_scroll: false,
            active_style: None,
            inactive_style: None,
            children: Vec::new(),
        }
    }

    #[must_use]
    pub fn search(mut self, search: serde_json::Value) -> Self {
        self.search = Some(search);
        self
    }

    #[must_use]
    pub fn hash(mut self, hash: impl Into<String>) -> Self {
        self.hash = Some(hash.into());
        self
    }

    #[must_use]
    pub fn replace(mut self, replace: bool) -> Self {
        self.replace = replace;
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[must_use]
    pub fn active_options(mut self, options: ActiveOptions) -> Self {
        self.active_options = options;
        self
    }

    #[must_use]
    pub fn reset_scroll(mut self, reset: bool) -> Self {
        self.reset_scroll = reset;
        self
    }

    /// Style hook applied when the link is active.
    #[must_use]
    pub fn active_style(mut self, f: impl Fn(NodeStyle) -> NodeStyle + 'static) -> Self {
        self.active_style = Some(Box::new(f));
        self
    }

    /// Style hook applied when the link is inactive.
    #[must_use]
    pub fn inactive_style(mut self, f: impl Fn(NodeStyle) -> NodeStyle + 'static) -> Self {
        self.inactive_style = Some(Box::new(f));
        self
    }

    #[must_use]
    pub fn preload(mut self, preload: PreloadType) -> Self {
        self.preload = Some(preload);
        self
    }

    /// Set a delay before intent-based preloading.
    #[must_use]
    pub fn preload_delay(mut self, delay: Duration) -> Self {
        self.preload_delay = Some(delay);
        self
    }

    #[must_use]
    pub fn child(mut self, child: impl Into<NodeChild>) -> Self {
        self.children.push(child.into().0);
        self
    }

    #[must_use]
    pub fn to(&self) -> &str {
        &self.href
    }

    #[must_use]
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    fn target_location(&self) -> crate::Location {
        let mut loc = crate::Location::new(&self.href);
        if let Some(s) = &self.search {
            loc.search.clone_from(s);
        }
        if let Some(h) = &self.hash {
            loc.hash.clone_from(h);
        }
        if let Some(st) = &self.state {
            loc.state.clone_from(st);
        }
        loc
    }

    /// Builds the link's node tree.
    #[must_use]
    pub fn build(self) -> Node {
        let href = self.href.clone();
        let disabled = self.disabled;

        // External links never match an app route: hand them to the OS and
        // skip router-specific behavior (active styles, preloads).
        if is_external_href(&href) {
            let mut builder = Node::div()
                .on_click(move |ctx: &mut quoin_ui::NodeEventCtx| {
                    if !disabled {
                        ctx.open_url(&href);
                    }
                })
                .children(self.children);
            if disabled {
                builder = builder.style(StyleCmd::Opacity(0.5));
            }
            return builder.build();
        }

        let replace = self.replace;
        let reset_scroll = self.reset_scroll;
        let target = self.target_location();
        let active_options = self.active_options.clone();

        let handle = RouterHandle::try_current();
        let is_active = handle.as_ref().is_some_and(|router_state| {
            let state = router_state.read();
            let current = state.current_location();
            let active = path_is_active(&current.pathname, &self.href, active_options.exact);
            let search_ok =
                !active_options.include_search || active && current.search == target.search;
            let hash_ok = !active_options.include_hash || active && current.hash == target.hash;
            active && search_ok && hash_ok
        });

        // Preload strategies. `Render`/`Viewport` kick off during this
        // build; `Intent` arms a hover handler that starts the preload
        // after a delay.
        match self.preload {
            Some(PreloadType::Render | PreloadType::Viewport) => {
                if !disabled
                    && handle.is_some()
                    && mark_preload_started(&href)
                    && let Some(handle) = &handle
                {
                    handle.update(|state| state.preload_location(&target.clone()));
                }
            }
            Some(PreloadType::Intent) | None => {}
        }

        let mut style = NodeStyle::new().with(StyleCmd::CursorPointer);
        if is_active {
            if let Some(f) = self.active_style {
                style = f(style);
            } else {
                style = style.with(StyleCmd::FontWeight(quoin_ui::NodeFontWeight::Bold));
            }
        } else if let Some(f) = self.inactive_style {
            style = f(style);
        }

        let mut builder = Node::div().styles(style.cmds).children(self.children);

        if self.preload == Some(PreloadType::Intent) {
            let delay = self.preload_delay.unwrap_or(Duration::from_millis(100));
            let intent_href = href.clone();
            let intent_target = target.clone();
            builder = builder.on_mouse_move(move |_ctx| {
                if disabled || !mark_preload_started(&intent_href) {
                    return;
                }
                let Some(handle) = RouterHandle::try_current() else {
                    return;
                };
                let intent_target = intent_target.clone();
                let spawn = handle.host().spawn;
                spawn(Box::pin(async move {
                    quoin_core::sleep(delay).await;
                    handle.update(|state| state.preload_location(&intent_target));
                }));
            });
        }

        builder = builder.on_click(move |_ctx| {
            if !disabled {
                let options = NavigateOptions {
                    reset_scroll: Some(reset_scroll),
                    ..NavigateOptions::default()
                };
                let navigator = Navigator::new();
                if replace {
                    navigator.replace_location_with(target.clone(), &options);
                } else {
                    navigator.push_location_with(target.clone(), &options);
                }
            }
        });

        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        assert!(path_is_active("/users", "/users", true));
        assert!(!path_is_active("/users/1", "/users", true));
        assert!(!path_is_active("/users-admin", "/users", true));
    }

    #[test]
    fn test_prefix_match_is_segment_aware() {
        assert!(path_is_active("/users/42", "/users", false));
        assert!(path_is_active("/users", "/users", false));
        assert!(!path_is_active("/users-admin", "/users", false));
        assert!(!path_is_active("/use", "/users", false));
    }

    #[test]
    fn test_root_is_always_active_prefix_mode() {
        assert!(path_is_active("/anything", "/", false));
        assert!(path_is_active("/", "/", false));
        assert!(!path_is_active("/x", "/", true));
    }

    #[test]
    fn test_preload_started_marker() {
        let key = format!("test-href-{}", std::process::id());
        assert!(mark_preload_started(&key), "first call starts preload");
        assert!(!mark_preload_started(&key), "second call is deduped");
    }

    #[test]
    fn test_external_href_detection() {
        assert!(is_external_href("https://example.com/docs"));
        assert!(is_external_href("http://example.com"));
        assert!(is_external_href("mailto:hi@example.com"));
        assert!(is_external_href("tel:+15550123456"));
        assert!(!is_external_href("/users"));
        assert!(!is_external_href("//example.com"));
        assert!(!is_external_href("users/42"));
    }
}
