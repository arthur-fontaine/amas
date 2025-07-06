use floem::{views::{canvas, Decorators}, IntoView};

use crate::{file::File, workspace_graph::WorkspaceGraph, workspace_layout::workspace_layout::WorkspaceLayout};

pub fn launch() {
    floem::launch(app_view);
}

fn app_view() -> impl IntoView {
    let mut graph = WorkspaceGraph::new();
    let a = graph.add_file(File {
        name: "A".to_string(),
    });
    let b = graph.add_file(File {
        name: "B".to_string(),
    });
    let c = graph.add_file(File {
        name: "C".to_string(),
    });
    let d = graph.add_file(File {
        name: "D".to_string(),
    });
    let e = graph.add_file(File {
        name: "E".to_string(),
    });
    graph.add_import(a, b);
    graph.add_import(b, c);
    graph.add_import(b, d);
    graph.add_import(a, e);
    graph.add_import(b, e);

    let layout = WorkspaceLayout::new(graph);

    canvas(move |cx, size| {
        layout.draw(cx, size);
    }).style(|s| s.size_full())
}
