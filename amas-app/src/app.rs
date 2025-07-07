use floem::{
    IntoView,
    views::{Decorators, stack},
};

use crate::{
    app_temp,
    editor::Editor,
    workspace_graph::{
        WorkspaceGraph, feeder::typescript::feed_workspace_graph_with_ts_project,
    },
    workspace_layout::workspace_layout::WorkspaceLayout,
};

pub fn launch() {
    floem::launch(app_view);
}

fn app_view() -> impl IntoView {
    let editor = Editor::new();

    let mut graph = WorkspaceGraph::new();
    feed_workspace_graph_with_ts_project(&mut graph, "/Users/arthur-fontaine/Developer/code/github.com/arthur-fontaine/mitosis-import-plugin").unwrap();

    let layout = WorkspaceLayout::new(graph, editor.clone());

    // This is a temporary hack to make launch in app_temp::app not marked as unused.
    if false {
        app_temp::app::launch()
    };

    stack((editor, layout)).style(|s| s.size_full())
}
