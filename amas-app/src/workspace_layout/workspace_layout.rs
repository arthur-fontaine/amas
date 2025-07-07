use super::view_state::ViewState;
use super::selection_state::SelectionState;
use super::canva_state::CanvaState;
use crate::workspace_graph::WorkspaceGraph;

#[derive(Clone, Debug)]
pub struct WorkspaceLayout {
    pub(super) workspace_graph: WorkspaceGraph,
    pub view_state: ViewState,
    pub selection_state: SelectionState,
    pub canva_state: CanvaState,
}

impl WorkspaceLayout {
    pub fn new(workspace_graph: WorkspaceGraph) -> Self {
        let view_state = ViewState::new();
        let selection_state = SelectionState::new();
        let canva_state = CanvaState::new();
        Self {
            workspace_graph,
            view_state,
            selection_state,
            canva_state,
        }
    }
}
