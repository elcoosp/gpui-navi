use std::sync::Arc;

#[derive(Clone)]
pub struct AnyData(pub Arc<dyn std::any::Any + Send + Sync>);

impl PartialEq for AnyData {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl std::fmt::Debug for AnyData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyData").finish()
    }
}
