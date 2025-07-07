use super::view_state::ViewState;
use crate::workspace_graph::WorkspaceGraph;

#[derive(Clone, Debug)]
pub struct WorkspaceLayout {
    pub(super) workspace_graph: WorkspaceGraph,
    pub view_state: ViewState,
}

impl WorkspaceLayout {
    pub fn new(workspace_graph: WorkspaceGraph) -> Self {
        let view_state = ViewState::new();
        Self {
            workspace_graph,
            view_state,
        }
    }
}
