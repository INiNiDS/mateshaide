use crate::shape::ShapeInstance;

const HISTORY_CAP: usize = 200;

pub(super) struct History {
    undo: Vec<Vec<ShapeInstance>>,
    redo: Vec<Vec<ShapeInstance>>,
}

impl History {
    pub(super) fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub(super) fn record_snapshot(&mut self, instances: &[ShapeInstance]) {
        self.undo.push(instances.to_vec());

        if self.undo.len() >= HISTORY_CAP {
            self.undo.remove(0);
        }

        self.redo.clear();
    }

    pub(super) fn undo(&mut self, instances: &mut Vec<ShapeInstance>) -> bool {
        if let Some(previous) = self.undo.pop() {
            let current = std::mem::replace(instances, previous);
            self.redo.push(current);
            true
        } else {
            false
        }
    }

    pub(super) fn redo(&mut self, instances: &mut Vec<ShapeInstance>) -> bool {
        if let Some(next) = self.redo.pop() {
            let current = std::mem::replace(instances, next);
            self.undo.push(current);
            true
        } else {
            false
        }
    }

    pub(super) fn discard_last_undo(&mut self) {
        self.undo.pop();
    }
}
