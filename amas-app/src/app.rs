use floem::{
    IntoView,
    event::{EventListener, EventPropagation},
    peniko::Color,
    prelude::{SignalGet as _, SignalUpdate as _, create_rw_signal},
    views::{Decorators, canvas, dyn_view, empty},
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
    let is_mouse_down = create_rw_signal(false);

    let point_x_at_start = create_rw_signal(0.0);
    let point_y_at_start = create_rw_signal(0.0);
    let translation_x = create_rw_signal(20.0);
    let translation_y = create_rw_signal(20.0);
    let translation_x_at_start = create_rw_signal(0.0);
    let translation_y_at_start = create_rw_signal(0.0);

    dyn_view(move || {
        let layout = layout.clone();
        let translation_x = translation_x.get();
        let translation_y = translation_y.get();
        canvas(move |cx, size| {
            layout.draw(cx, size);
        })
        .style(move |s| {
            s.size_full()
                .translate_x(translation_x)
                .translate_y(translation_y)
        })
    })
    .style(move |s| s.size_full())
    .on_event(EventListener::PointerDown, {
        let is_mouse_down = is_mouse_down.clone();
        move |event| {
            is_mouse_down.set(true);
            if let Some(pointer_position) = event.point() {
                point_x_at_start.set(pointer_position.x);
                point_y_at_start.set(pointer_position.y);
                translation_x_at_start.set(translation_x.get());
                translation_y_at_start.set(translation_y.get());
            }
            EventPropagation::Continue
        }
    })
    .on_event(EventListener::PointerUp, {
        let is_mouse_down = is_mouse_down.clone();
        move |_event| {
            is_mouse_down.set(false);
            EventPropagation::Continue
        }
    })
    .on_event(EventListener::PointerMove, {
        let is_mouse_down = is_mouse_down.clone();
        move |event| {
            if is_mouse_down.get() {
                // Handle mouse move while down
                if let Some(pointer_position) = event.point() {
                    let start_x = point_x_at_start.get();
                    let start_y = point_y_at_start.get();
                    let delta_x = pointer_position.x - start_x;
                    let delta_y = pointer_position.y - start_y;
                    let translation_x_at_start_value = translation_x_at_start.get();
                    let translation_y_at_start_value = translation_y_at_start.get();
                    translation_x.set(translation_x_at_start_value + delta_x);
                    translation_y.set(translation_y_at_start_value + delta_y);
                }
            }
            EventPropagation::Continue
        }
    })
}
