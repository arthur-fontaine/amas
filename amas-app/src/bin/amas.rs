#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use amas_app::app;

pub fn main() {
    app::launch();
}
