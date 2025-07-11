use crate::app_temp;

use super::editor::Editor;
use floem::unit::{DurationUnitExt, UnitExt};
use floem::{
    AnyView, IntoView,
    event::{Event, EventListener, EventPropagation},
    views::{Decorators, dyn_container},
};
use lapce_rpc::file::LineCol;

impl IntoView for Editor {
    type V = AnyView;

    fn into_view(self) -> Self::V {
        dyn_container(
            {
                let editor = self.clone();
                move || editor.get_opened_file()
            },
            {
                let editor = self.clone();
                move |file_name| {
                    if let Some(file_name) = file_name {
                        app_temp::app::into_view(
                            self.window_id,
                            &file_name,
                            Some(LineCol { line: 0, column: 0 }),
                        )
                        .style(|s| {
                            s.size_full()
                                .scale(100.pct())
                                .translate_x(0)
                                .translate_y(0)
                        })
                        .animation(move |a| {
                            a.duration(200.millis())
                                .keyframe(0, |f| {
                                    f.computed_style()
                                        .style(|s| s.scale(0))
                                        .ease_in()
                                })
                                .keyframe(100, |f| f.computed_style().ease_out())
                                .run_on_create(true)
                                .only_on_create()
                        })
                        .on_event(EventListener::PinchGesture, {
                            let editor = editor.clone();
                            move |event| {
                                if let Event::PinchGesture(pinch_event) = event {
                                    // When we zoomed at maximum zoom level, we can open the file we are hovering over
                                    if pinch_event.delta < 0.0 {
                                        let editor = editor.clone();
                                        editor.close_file(&file_name);
                                    }
                                }
                                EventPropagation::Continue
                            }
                        })
                        .into_any()
                    } else {
                        "<no file opened>".into_any()
                    }
                }
            },
        )
        .style(|s| s.absolute().size_full().z_index(9))
        .into_any()
    }
}
