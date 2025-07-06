use crate::file::File;

#[derive(Debug, Clone)]
pub struct WorkspaceGraph {
    pub graph: petgraph::Graph<File, f64, petgraph::Undirected>,
}

impl WorkspaceGraph {
    pub fn new() -> Self {
        WorkspaceGraph {
            graph: petgraph::Graph::new_undirected(),
        }
    }

    pub fn add_file(&mut self, file: File) -> petgraph::graph::NodeIndex {
        self.graph.add_node(file)
    }

    pub fn add_import(&mut self, a: petgraph::graph::NodeIndex, b: petgraph::graph::NodeIndex) {
        self.graph.add_edge(a, b, 1.0);
    }
}
