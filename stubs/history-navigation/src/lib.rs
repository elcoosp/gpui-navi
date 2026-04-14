//! Stub crate for history-navigation - Browser history management

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

    pub fn back(&mut self) -> bool {
        if self.index > 0 {
            self.index -= 1;
            true
        } else {
            false
        }
    }

    pub fn forward(&mut self) -> bool {
        if self.index + 1 < self.entries.len() {
            self.index += 1;
            true
        } else {
            false
        }
    }

    pub fn go(&mut self, delta: isize) {
        let new_index = self.index as isize + delta;
        if new_index >= 0 && (new_index as usize) < self.entries.len() {
            self.index = new_index as usize;
        }
    }

    pub fn current(&self) -> &T {
        &self.entries[self.index]
    }

    pub fn can_go_back(&self) -> bool {
        self.index > 0
    }

    pub fn can_go_forward(&self) -> bool {
        self.index + 1 < self.entries.len()
    }
}
