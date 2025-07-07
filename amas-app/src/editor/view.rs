use super::editor::Editor;
use floem::{
    IntoView,
    views::{Empty, empty},
};

impl IntoView for Editor {
    type V = Empty;

    fn into_view(self) -> Self::V {
        empty()
    }
}
