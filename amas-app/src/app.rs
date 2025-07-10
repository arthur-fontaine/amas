use floem::{
    Application, IntoView,
    views::{Decorators, stack},
    window::{WindowConfig, WindowId},
};

use crate::{
    editor::Editor,
    workspace_graph::{
        WorkspaceGraph, feeder::typescript::feed_workspace_graph_with_ts_project,
    },
    workspace_layout::workspace_layout::WorkspaceLayout,
};

pub fn launch() {
    Application::new()
        .window(app_view, Some(WindowConfig::default()))
        .run();
}

fn app_view(window_id: WindowId) -> impl IntoView {
    let editor = Editor::new(window_id);

    let mut graph = WorkspaceGraph::new();
    feed_workspace_graph_with_ts_project(&mut graph, "/Users/arthur-fontaine/Developer/code/github.com/arthur-fontaine/mitosis-import-plugin").unwrap();

    let layout = WorkspaceLayout::new(graph, editor.clone());

    stack((editor, layout)).style(|s| s.size_full())
}
