use floem::{
    IntoView,
    event::{Event, EventListener, EventPropagation},
    views::{Decorators, canvas, dyn_view},
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
    feed_workspace_graph_with_ts_project(&mut graph, "/Users/arthur-fontaine/Developer/code/github.com/arthur-fontaine/mitosis-import-plugin").unwrap();

    let layout = WorkspaceLayout::new(graph);

    dyn_view({
        let layout = layout.clone();
        move || {
            canvas({
                let layout = layout.clone();
                move |cx, size| {
                    layout.draw(cx, size);
                }
            })
            .style(move |s| s.size_full())
        }
    })
    .style(move |s| s.size_full())
    .on_event(EventListener::PointerDown, {
        let layout = layout.clone();
        move |_event| {
            layout.start_mouse_drag();
            EventPropagation::Continue
        }
    })
    .on_event(EventListener::PointerUp, {
        let layout = layout.clone();
        move |_event| {
            layout.end_mouse_drag();
            EventPropagation::Continue
        }
    })
    .on_event(EventListener::PointerMove, {
        let layout = layout.clone();
        move |event| {
            if let Some(pointer_position) = event.point() {
                layout.track_mouse_position(pointer_position.x, pointer_position.y);
                layout.track_hovered_file(pointer_position.x, pointer_position.y);
            }
            EventPropagation::Continue
        }
    })
    .on_event(EventListener::PinchGesture, {
        let layout = layout.clone();
        move |event| {
            if let Event::PinchGesture(pinch_event) = event {
                layout.zoom(pinch_event.delta);
            }
            EventPropagation::Continue
        }
    })
    .on_event(EventListener::PointerWheel, {
        let layout = layout.clone();
        move |event| {
            if let Event::PointerWheel(pointer_wheel_event) = event {
                layout
                    .move_(pointer_wheel_event.delta.x, pointer_wheel_event.delta.y);
            }
            EventPropagation::Continue
        }
    })
    .on_event(EventListener::Click, {
        let layout = layout.clone();
        move |_event| {
            layout.select_file_hovered_file();
            EventPropagation::Continue
        }
    })
}
