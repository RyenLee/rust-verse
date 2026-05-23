// Prevents additional console window on Windows in release builds, release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    rustverse_lib::run()
}
