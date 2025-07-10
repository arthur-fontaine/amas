use crate::app_temp;

use super::editor::Editor;
use floem::{
    AnyView, IntoView,
    views::{Decorators, dyn_container},
};
use lapce_rpc::file::LineCol;

impl IntoView for Editor {
    type V = AnyView;

    fn into_view(self) -> Self::V {
        let editor = self.clone();
        dyn_container(
            move || editor.get_opened_file(),
            move |file_name| {
                if let Some(file_name) = file_name {
                    app_temp::app::into_view(
                        self.window_id,
                        &file_name,
                        Some(LineCol { line: 0, column: 0 }),
                    )
                    .style(|s| s.size_full())
                    .into_any()
                } else {
                    "<no file opened>".into_any()
                }
            },
        )
        .style(|s| s.absolute().size_full().z_index(9))
        .into_any()
    }
}
