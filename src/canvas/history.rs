use crate::canvas::text::CanvasText;
use crate::shape::ShapeInstance;
use std::collections::VecDeque;

const HISTORY_LIMIT: usize = 200;

#[derive(Clone, PartialEq)]
struct Snapshot {
    instances: Vec<ShapeInstance>,
    texts: Vec<CanvasText>,
    nodes: Vec<crate::plugins::PluginNode>,
}

#[derive(Default)]
pub struct History {
    undo: VecDeque<Snapshot>,
    redo: Vec<Snapshot>,
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }
    pub(super) fn record_snapshot(
        &mut self,
        instances: &[ShapeInstance],
        texts: &[CanvasText],
        nodes: &[crate::plugins::PluginNode],
    ) -> bool {
        let snapshot = Snapshot {
            instances: instances.to_vec(),
            texts: texts.to_vec(),
            nodes: nodes.to_vec(),
        };
        if self.undo.back() == Some(&snapshot) {
            return false;
        }
        if self.undo.len() == HISTORY_LIMIT {
            self.undo.pop_front();
        }
        self.undo.push_back(snapshot);
        self.redo.clear();
        true
    }
    pub(super) fn discard_last_undo(&mut self) {
        self.undo.pop_back();
    }
    pub(super) fn undo(
        &mut self,
        instances: &mut Vec<ShapeInstance>,
        texts: &mut Vec<CanvasText>,
        nodes: &mut Vec<crate::plugins::PluginNode>,
    ) -> bool {
        let Some(snapshot) = self.undo.pop_back() else {
            return false;
        };
        self.redo.push(Snapshot {
            instances: std::mem::replace(instances, snapshot.instances),
            texts: std::mem::replace(texts, snapshot.texts),
            nodes: std::mem::replace(nodes, snapshot.nodes),
        });
        true
    }
    pub(super) fn redo(
        &mut self,
        instances: &mut Vec<ShapeInstance>,
        texts: &mut Vec<CanvasText>,
        nodes: &mut Vec<crate::plugins::PluginNode>,
    ) -> bool {
        let Some(snapshot) = self.redo.pop() else {
            return false;
        };
        if self.undo.len() == HISTORY_LIMIT {
            self.undo.pop_front();
        }
        self.undo.push_back(Snapshot {
            instances: std::mem::replace(instances, snapshot.instances),
            texts: std::mem::replace(texts, snapshot.texts),
            nodes: std::mem::replace(nodes, snapshot.nodes),
        });
        true
    }
}
