#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub fn main() {
    amas_app::app_temp::app::launch();
}
