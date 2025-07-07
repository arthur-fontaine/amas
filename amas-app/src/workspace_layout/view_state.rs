use floem::prelude::{RwSignal, SignalGet as _, SignalUpdate as _};

use crate::workspace_layout::workspace_layout::WorkspaceLayout;

#[derive(Clone, Debug)]
pub struct ViewState {
    pub zoom: RwSignal<f64>,
    pub translation_x: RwSignal<f64>,
    pub translation_y: RwSignal<f64>,

    // Drag internal states
    drag_started: RwSignal<bool>,
    drag_start_x: RwSignal<f64>,
    drag_start_y: RwSignal<f64>,
    // Mouse tracking internal states
    mouse_position_x: RwSignal<f64>,
    mouse_position_y: RwSignal<f64>,
}

impl ViewState {
    pub fn new() -> Self {
        let zoom = RwSignal::new(1.0);
        let translation_x = RwSignal::new(0.0);
        let translation_y = RwSignal::new(0.0);

        let drag_started = RwSignal::new(false);
        let drag_start_x = RwSignal::new(0.0);
        let drag_start_y = RwSignal::new(0.0);

        let mouse_position_x = RwSignal::new(0.0);
        let mouse_position_y = RwSignal::new(0.0);

        Self {
            zoom,
            translation_x,
            translation_y,
            // Internal states
            drag_started,
            drag_start_x,
            drag_start_y,
            mouse_position_x,
            mouse_position_y,
        }
    }
}

impl WorkspaceLayout {
    pub fn track_mouse_position(&self, x: f64, y: f64) {
        self.view_state.mouse_position_x.set(x);
        self.view_state.mouse_position_y.set(y);

        self.mouse_drag(x, y);
    }

    pub fn start_mouse_drag(&self) {
        self.view_state.drag_started.set(true);
        self.view_state
            .drag_start_x
            .set(self.view_state.mouse_position_x.get());
        self.view_state
            .drag_start_y
            .set(self.view_state.mouse_position_y.get());
    }

    fn mouse_drag(&self, current_x: f64, current_y: f64) {
        if !self.view_state.drag_started.get() {
            return; // Drag not started, ignore the event
        }

        let start_x = self.view_state.drag_start_x.get();
        let start_y = self.view_state.drag_start_y.get();

        let delta_x = current_x - start_x;
        let delta_y = current_y - start_y;

        self.view_state.translation_x.update(|x| *x += delta_x);
        self.view_state.translation_y.update(|y| *y += delta_y);

        // Update the drag start position for the next event
        self.view_state.drag_start_x.set(current_x);
        self.view_state.drag_start_y.set(current_y);
    }

    pub fn end_mouse_drag(&self) {
        self.view_state.drag_started.set(false);
    }

    pub fn zoom(&self, factor: f64) {
        let mouse_x = self.view_state.mouse_position_x.get();
        let mouse_y = self.view_state.mouse_position_y.get();
        let prev_zoom = self.view_state.zoom.get();
        let new_zoom = (prev_zoom + factor).clamp(0.1, 10.0);
        self.view_state.zoom.set(new_zoom);

        let tx = self.view_state.translation_x.get();
        let ty = self.view_state.translation_y.get();

        let scale = new_zoom / prev_zoom;
        let new_tx = (tx - mouse_x) * scale + mouse_x;
        let new_ty = (ty - mouse_y) * scale + mouse_y;

        self.view_state.translation_x.set(new_tx);
        self.view_state.translation_y.set(new_ty);
    }

    pub fn move_(&self, dx: f64, dy: f64) {
        self.view_state.translation_x.update(|x| *x += dx);
        self.view_state.translation_y.update(|y| *y += dy);
    }
}
