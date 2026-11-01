//! Stub crate for `history-navigation` — browser-style history management.

pub struct History<T> {
    entries: Vec<T>,
    index: usize,
}

impl<T: Clone> History<T> {
    pub fn new(initial: T) -> Self {
        Self {
            entries: vec![initial],
            index: 0,
        }
    }

    pub fn push(&mut self, entry: T) {
        self.entries.truncate(self.index + 1);
        self.entries.push(entry);
        self.index = self.entries.len() - 1;
    }

    pub fn replace(&mut self, entry: T) {
        if self.entries.is_empty() {
            self.entries.push(entry);
            self.index = 0;
        } else {
            self.entries[self.index] = entry;
        }
    }

    #[must_use]
    pub fn back(&mut self) -> bool {
        if self.index > 0 {
            self.index -= 1;
            true
        } else {
            false
        }
    }

    #[must_use]
    pub fn forward(&mut self) -> bool {
        if self.index + 1 < self.entries.len() {
            self.index += 1;
            true
        } else {
            false
        }
    }

    pub fn go(&mut self, delta: isize) {
        // Browser semantics: navigating past either end clamps to the
        // first/last entry instead of being ignored. `checked_add_signed`
        // keeps the arithmetic cast-free.
        let len = self.entries.len();
        if len == 0 {
            return;
        }
        self.index = match self.index.checked_add_signed(delta) {
            Some(index) if index < len => index,
            Some(_) => len - 1,
            None => 0,
        };
    }

    #[must_use]
    pub fn current(&self) -> &T {
        &self.entries[self.index]
    }

    #[must_use]
    pub fn can_go_back(&self) -> bool {
        self.index > 0
    }

    #[must_use]
    pub fn can_go_forward(&self) -> bool {
        self.index + 1 < self.entries.len()
    }

    /// Number of entries currently stored.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the history has no entries (never the case after `new`).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h() -> History<i32> {
        History::new(0)
    }

    #[test]
    fn test_new_starts_at_initial() {
        let history = h();
        assert_eq!(*history.current(), 0);
        assert_eq!(history.len(), 1);
        assert!(!history.is_empty());
    }

    #[test]
    fn test_push_moves_forward() {
        let mut history = h();
        history.push(1);
        history.push(2);
        assert_eq!(*history.current(), 2);
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn test_back_forward() {
        let mut history = h();
        history.push(1);
        assert!(history.back());
        assert_eq!(*history.current(), 0);
        assert!(history.forward());
        assert_eq!(*history.current(), 1);
        assert!(!history.forward());
        assert!(history.back(), "can go back again after forward");
        assert_eq!(*history.current(), 0);
    }

    #[test]
    fn test_replace_keeps_position() {
        let mut history = h();
        history.push(1);
        history.replace(9);
        assert_eq!(*history.current(), 9);
        assert_eq!(history.len(), 2);
        assert!(history.can_go_back());
    }

    #[test]
    fn test_push_truncates_redo_stack() {
        let mut history = h();
        history.push(1);
        history.push(2);
        assert!(history.back());
        history.push(3);
        assert_eq!(*history.current(), 3);
        assert!(!history.can_go_forward());
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn test_go_clamps_to_bounds() {
        let mut history = h();
        history.push(1);
        history.push(2);
        history.go(-10);
        assert_eq!(*history.current(), 0);
        history.go(10);
        assert_eq!(*history.current(), 2);
        history.go(0);
        assert_eq!(*history.current(), 2);
    }

    #[test]
    fn test_can_go_flags() {
        let mut history = h();
        assert!(!history.can_go_back());
        assert!(!history.can_go_forward());
        history.push(1);
        assert!(history.can_go_back());
        assert!(!history.can_go_forward());
    }

    #[test]
    fn test_replace_on_empty() {
        let mut history: History<u8> = History {
            entries: Vec::new(),
            index: 0,
        };
        history.replace(7);
        assert_eq!(*history.current(), 7);
        assert_eq!(history.len(), 1);
    }
}
