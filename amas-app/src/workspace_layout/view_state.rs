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
        let old_zoom = self.view_state.zoom.get();
        let new_zoom = (old_zoom + factor).clamp(0.1, 3.0);

        if old_zoom == new_zoom {
            return;
        }

        let old_tx = self.view_state.translation_x.get();
        let old_ty = self.view_state.translation_y.get();

        // Calculate the point in world space under the mouse
        let world_x = (mouse_x - old_tx) / old_zoom;
        let world_y = (mouse_y - old_ty) / old_zoom;

        // Calculate new translation to keep the same world point under the mouse
        let new_tx = mouse_x - world_x * new_zoom;
        let new_ty = mouse_y - world_y * new_zoom;

        self.view_state.zoom.set(new_zoom);
        self.view_state.translation_x.set(new_tx);
        self.view_state.translation_y.set(new_ty);
    }

    pub fn move_(&self, dx: f64, dy: f64) {
        self.view_state.translation_x.update(|x| *x += dx);
        self.view_state.translation_y.update(|y| *y += dy);
    }
}
