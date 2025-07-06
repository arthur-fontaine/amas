use crate::workspace_graph::WorkspaceGraph;

pub struct WorkspaceLayout {
    pub(super) workspace_graph: WorkspaceGraph,
}

impl WorkspaceLayout {
    pub fn new(workspace_graph: WorkspaceGraph) -> Self {
        Self { workspace_graph }
    }
}
