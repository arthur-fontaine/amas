use floem::{
    IntoView,
    event::{Event, EventListener, EventPropagation},
    peniko::Color,
    prelude::{SignalGet as _, SignalUpdate as _, create_rw_signal},
    touchpad::PinchGestureEvent,
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
    let zoom = create_rw_signal(1.0);
    let mouse_position_x = create_rw_signal(0.0);
    let mouse_position_y = create_rw_signal(0.0);

    dyn_view(move || {
        let layout = layout.clone();
        let translation_x = translation_x.get();
        let translation_y = translation_y.get();
        let zoom = zoom.get();
        canvas(move |cx, size| {
            layout.draw(cx, size);
        })
        .style(move |s| {
            s.size_full()
                .translate_x(translation_x)
                .translate_y(translation_y)
                .scale(zoom * 100.0)
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
            if let Some(pointer_position) = event.point() {
                mouse_position_x.set(pointer_position.x);
                mouse_position_y.set(pointer_position.y);
                if is_mouse_down.get() {
                    // Handle mouse move while down
                    let start_x: f64 = point_x_at_start.get();
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
    .on_event(EventListener::PinchGesture, {
        move |event| {
            if let Event::PinchGesture(pinch_event) = event {
                zoom.set(zoom.get() + pinch_event.delta as f32);
                let mouse_x = mouse_position_x.get();
                let mouse_y = mouse_position_y.get();
                let prev_zoom = zoom.get();
                let new_zoom = prev_zoom + pinch_event.delta as f32;
                let tx = translation_x.get();
                let ty = translation_y.get();

                // Calculate the new translation so that the zoom centers on the mouse position
                let scale = new_zoom / prev_zoom;
                let new_tx = (tx - mouse_x) * scale as f64 + mouse_x;
                let new_ty = (ty - mouse_y) * scale as f64 + mouse_y;

                translation_x.set(new_tx);
                translation_y.set(new_ty);
            }
            EventPropagation::Continue
        }
    })
}
