use super::workspace_layout::WorkspaceLayout;
use floem::{
    IntoView,
    event::{Event, EventListener, EventPropagation},
    prelude::SignalGet as _,
    views::{Decorators as _, DynamicView, canvas, dyn_view},
};

impl IntoView for WorkspaceLayout {
    type V = DynamicView;

    fn into_view(self) -> Self::V {
        let editor = self.editor.clone();
        let layout = self.clone();

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
                    layout.track_mouse_position(
                        pointer_position.x,
                        pointer_position.y,
                    );
                    layout
                        .track_hovered_file(pointer_position.x, pointer_position.y);
                }
                EventPropagation::Continue
            }
        })
        .on_event(EventListener::PinchGesture, {
            let editor = editor.clone();
            let layout = layout.clone();
            move |event| {
                if let Event::PinchGesture(pinch_event) = event {
                    layout.zoom(pinch_event.delta);

                    // When we zoomed at maximum zoom level, we can open the file we are hovering over
                    if layout.view_state.zoom.get() == 3.5 && pinch_event.delta > 0.0
                    {
                        let editor = editor.clone();
                        layout.get_hovered_file().map(|file_name| {
                            editor.open_file(&file_name);
                        });
                    }
                }
                EventPropagation::Continue
            }
        })
        .on_event(EventListener::PointerWheel, {
            let layout = layout.clone();
            move |event| {
                if let Event::PointerWheel(pointer_wheel_event) = event {
                    layout.move_(
                        pointer_wheel_event.delta.x,
                        pointer_wheel_event.delta.y,
                    );
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
        .on_event(EventListener::DoubleClick, {
            let editor = editor.clone();
            let layout = layout.clone();
            move |_event| {
                let editor = editor.clone();
                layout.get_hovered_file().map(|file_name| {
                    editor.open_file(&file_name);
                });
                EventPropagation::Continue
            }
        })
    }
}
