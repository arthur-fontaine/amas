use super::editor::Editor;
use floem::{
    IntoView,
    views::{Decorators, DynamicView, dyn_view},
};

impl IntoView for Editor {
    type V = DynamicView;

    fn into_view(self) -> Self::V {
        dyn_view(move || self.get_opened_file().unwrap_or("<no file opened>".into()))
            .style(|s| s.absolute())
    }
}
