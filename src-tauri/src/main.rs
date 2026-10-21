// Sin consola en compilaciones de producción de Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    wispr_swag_lib::run();
}
