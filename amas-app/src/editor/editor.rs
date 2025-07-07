use floem::prelude::{
    RwSignal, SignalGet as _, SignalUpdate as _, create_rw_signal,
};

#[derive(Debug, Clone)]
pub struct Editor {
    opened_files: RwSignal<Vec<String>>,
}

impl Editor {
    pub fn new() -> Self {
        let opened_files = create_rw_signal(Vec::new());
        Self { opened_files }
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
