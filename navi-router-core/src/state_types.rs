use std::any::Any;
use std::sync::Arc;

/// Type-erased, cloneable data holder used for loader payloads and context.
#[derive(Clone)]
pub struct AnyData(pub Arc<dyn Any + Send + Sync>);

impl AnyData {
    /// Wrap an arbitrary value.
    #[must_use]
    pub fn new<T: Any + Send + Sync>(value: T) -> Self {
        Self(Arc::new(value))
    }

    /// Borrow the inner value if it is of type `T`.
    #[must_use]
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.0.downcast_ref::<T>()
    }

    /// Clone out the inner value if it is of type `T`.
    #[must_use]
    pub fn get_cloned<T: Any + Clone>(&self) -> Option<T> {
        self.downcast_ref::<T>().cloned()
    }
}

impl PartialEq for AnyData {
    fn eq(&self, _other: &Self) -> bool {
        // Type-erased data has no meaningful structural equality.
        false
    }
}

impl std::fmt::Debug for AnyData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyData").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_downcast() {
        let data = AnyData::new(42_u32);
        assert_eq!(data.downcast_ref::<u32>(), Some(&42));
        assert!(data.downcast_ref::<u64>().is_none());
    }

    #[test]
    fn test_get_cloned() {
        let data = AnyData::new("hello".to_string());
        assert_eq!(data.get_cloned::<String>(), Some("hello".to_string()));
        assert!(data.get_cloned::<u8>().is_none());
    }

    #[test]
    fn test_clone_shares_inner() {
        let data = AnyData::new(vec![1_u8, 2, 3]);
        let cloned = data.clone();
        assert_eq!(cloned.downcast_ref::<Vec<u8>>(), Some(&vec![1_u8, 2, 3]));
    }

    #[test]
    fn test_partial_eq_always_false() {
        let a = AnyData::new(1_u8);
        let b = AnyData::new(1_u8);
        assert_ne!(a, b);
    }

    #[test]
    fn test_debug_is_opaque() {
        let data = AnyData::new(1_u8);
        assert_eq!(format!("{data:?}"), "AnyData");
    }

    #[test]
    fn test_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AnyData>();
    }
}
