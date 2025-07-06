use crate::workspace_graph::WorkspaceGraph;

#[derive(Clone, Debug)]
pub struct WorkspaceLayout {
    pub(super) workspace_graph: WorkspaceGraph,
}

impl WorkspaceLayout {
    pub fn new(workspace_graph: WorkspaceGraph) -> Self {
        Self { workspace_graph }
    }
}
