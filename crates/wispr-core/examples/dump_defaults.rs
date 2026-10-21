//! Imprime los ajustes por defecto en JSON (usado por la vista previa de la interfaz).
fn main() {
    println!("{}", serde_json::to_string_pretty(&wispr_core::settings::Settings::default()).unwrap());
}
