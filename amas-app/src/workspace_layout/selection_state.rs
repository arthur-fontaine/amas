use std::collections::HashSet;

use super::workspace_layout::WorkspaceLayout;
use floem::prelude::{RwSignal, SignalGet as _, SignalUpdate};

#[derive(Clone, Debug)]
pub struct SelectionState {
    pub selected_files: RwSignal<HashSet<String>>,
    pub hovered_file: RwSignal<Option<String>>,
}

impl SelectionState {
    pub fn new() -> Self {
        let selected_files = RwSignal::new(HashSet::new());
        let hovered_file = RwSignal::new(None);
        Self {
            selected_files,
            hovered_file,
        }
    }
}

impl WorkspaceLayout {
    fn get_file_at_position(&self, x: f64, y: f64) -> Option<String> {
        self.canva_state
            .files
            .get()
            .iter()
            .find_map(|(file, (fx, fy, fw, fh))| {
                if x >= *fx && x <= *fw && y >= *fy && y <= *fh {
                    Some(file.name.clone())
                } else {
                    None
                }
            })
    }

    pub fn select_file_hovered_file(&self) {
        if let Some(file_name) = self.selection_state.hovered_file.get().clone() {
            self.selection_state
                .selected_files
                .set(HashSet::from([file_name.clone()]));
        } else {
            self.selection_state.selected_files.set(HashSet::new());
        }
    }

    pub fn multiselect_files_hovered_file(&self) {
        if let Some(file_name) = self.selection_state.hovered_file.get().clone() {
            self.selection_state
                .selected_files
                .update(|selected_files| {
                    if selected_files.contains(&file_name) {
                        selected_files.remove(&file_name);
                    } else {
                        selected_files.insert(file_name);
                    }
                });
        }
    }

    pub fn track_hovered_file(&self, x: f64, y: f64) {
        self.selection_state
            .hovered_file
            .set(self.get_file_at_position(x, y));
    }

    pub fn get_hovered_file(&self) -> Option<String> {
        self.selection_state.hovered_file.get().clone()
    }

    pub fn get_selected_files(&self) -> HashSet<String> {
        self.selection_state.selected_files.get().clone()
    }
}
