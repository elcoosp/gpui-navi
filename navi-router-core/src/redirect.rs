use crate::location::NavigateOptions;

#[derive(Clone, PartialEq)]
pub struct Redirect { pub to: String, pub options: NavigateOptions }

impl Redirect {
    pub fn replace(mut self) -> Self { self.options.replace = true; self }
    pub fn reload_document(mut self) -> Self { self.options.reload_document = true; self }
}

pub fn redirect(to: impl Into<String>) -> Redirect { Redirect { to: to.into(), options: NavigateOptions::default() } }

#[derive(Clone, PartialEq)]
pub struct NotFound { pub route_id: Option<String>, pub data: Option<serde_json::Value> }

impl NotFound {
    pub fn with_route_id(mut self, id: impl Into<String>) -> Self { self.route_id = Some(id.into()); self }
    pub fn with_data(mut self, d: serde_json::Value) -> Self { self.data = Some(d); self }
}

pub fn not_found() -> NotFound { NotFound { route_id: None, data: None } }
