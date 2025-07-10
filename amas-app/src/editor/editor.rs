use floem::{prelude::{
    create_rw_signal, RwSignal, SignalGet as _, SignalUpdate as _
}, window::WindowId};

#[derive(Debug, Clone)]
pub struct Editor {
    opened_files: RwSignal<Vec<String>>,
    pub(super) window_id: WindowId,
}

impl Editor {
    pub fn new(window_id: WindowId) -> Self {
        let opened_files = create_rw_signal(Vec::new());
        Self { opened_files, window_id }
    }

    pub fn open_file(&self, file_name: &str) {
        // For now, we just support opening a single file. Maybe later it will be useful to support multiple opened files.
        // opened_files is a vector in case we want to support multiple opened files in the future.
        self.opened_files.set(vec![file_name.to_string()]);
    }

    pub fn get_opened_file(&self) -> Option<String> {
        self.opened_files.get().first().cloned()
    }
}
