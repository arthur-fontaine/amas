use floem::{
    IntoView,
    views::{Decorators, canvas},
};

use crate::{
    workspace_graph::{
        WorkspaceGraph, feeder::typescript::feed_workspace_graph_with_ts_project,
    },
    workspace_layout::workspace_layout::WorkspaceLayout,
};

pub fn launch() {
    floem::launch(app_view);
}

fn app_view() -> impl IntoView {
    let mut graph = WorkspaceGraph::new();
    feed_workspace_graph_with_ts_project(&mut graph, "/Users/arthurfontaine/Developer/code/github.com/arthur-fontaine/agrume/packages/agrume").unwrap();

    let layout = WorkspaceLayout::new(graph);

    canvas(move |cx, size| {
        layout.draw(cx, size);
    })
    .style(|s| s.size_full())
}
