use floem::prelude::{RwSignal, SignalUpdate as _};

use crate::file::File;

#[derive(Clone, Debug)]
pub struct CanvaState {
    pub files: RwSignal<Vec<(File, (f64, f64, f64, f64))>>,
}

impl CanvaState {
    pub fn new() -> Self {
        let files = RwSignal::new(Vec::new());
        Self { files }
    }

    pub fn set_files(&self, files: Vec<(File, (f64, f64, f64, f64))>) {
        self.files.set(files);
    }
}
