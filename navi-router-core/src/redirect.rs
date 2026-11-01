use crate::location::NavigateOptions;

#[derive(Clone, Debug, PartialEq)]
pub struct Redirect {
    pub to: String,
    pub options: NavigateOptions,
}

impl Redirect {
    #[must_use]
    pub fn replace(mut self) -> Self {
        self.options.replace = true;
        self
    }

    #[must_use]
    pub fn reload_document(mut self) -> Self {
        self.options.reload_document = true;
        self
    }
}

#[must_use]
pub fn redirect(to: impl Into<String>) -> Redirect {
    Redirect {
        to: to.into(),
        options: NavigateOptions::default(),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NotFound {
    pub route_id: Option<String>,
    pub data: Option<serde_json::Value>,
}

impl NotFound {
    #[must_use]
    pub fn with_route_id(mut self, id: impl Into<String>) -> Self {
        self.route_id = Some(id.into());
        self
    }

    #[must_use]
    pub fn with_data(mut self, d: serde_json::Value) -> Self {
        self.data = Some(d);
        self
    }
}

#[must_use]
pub fn not_found() -> NotFound {
    NotFound {
        route_id: None,
        data: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redirect_basic() {
        let r = redirect("/login");
        assert_eq!(r.to, "/login");
        assert!(!r.options.replace);
        assert!(!r.options.reload_document);
    }

    #[test]
    fn test_redirect_accepts_string_and_str() {
        let owned: String = "/owned".to_string();
        assert_eq!(redirect(owned).to, "/owned");
        assert_eq!(redirect("/static").to, "/static");
    }

    #[test]
    fn test_redirect_replace_builder() {
        let r = redirect("/home").replace();
        assert!(r.options.replace);
        assert_eq!(r.to, "/home");
    }

    #[test]
    fn test_redirect_reload_document_builder() {
        let r = redirect("/home").reload_document();
        assert!(r.options.reload_document);
    }

    #[test]
    fn test_redirect_chained_builders() {
        let r = redirect("/x").replace().reload_document();
        assert!(r.options.replace && r.options.reload_document);
    }

    #[test]
    fn test_not_found_defaults() {
        let nf = not_found();
        assert_eq!(nf.route_id, None);
        assert_eq!(nf.data, None);
    }

    #[test]
    fn test_not_found_builders() {
        let nf = not_found()
            .with_route_id("users")
            .with_data(serde_json::json!({"msg": "no user"}));
        assert_eq!(nf.route_id.as_deref(), Some("users"));
        assert_eq!(nf.data, Some(serde_json::json!({"msg": "no user"})));
    }

    #[test]
    fn test_not_found_route_id_accepts_string() {
        let nf = not_found().with_route_id(String::from("docs"));
        assert_eq!(nf.route_id.as_deref(), Some("docs"));
    }

    #[test]
    fn test_redirect_partial_eq() {
        assert_eq!(redirect("/a"), redirect("/a"));
        assert_ne!(redirect("/a"), redirect("/b"));
        assert_ne!(redirect("/a"), redirect("/a").replace());
    }

    #[test]
    fn test_not_found_partial_eq() {
        assert_eq!(not_found(), not_found());
        assert_ne!(not_found(), not_found().with_route_id("x"));
    }
}
