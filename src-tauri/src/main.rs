// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    env_logger::Builder::from_default_env()
        .filter_module("mvt", log::LevelFilter::Info)
        .init();
    chuck_lib::run()
}
