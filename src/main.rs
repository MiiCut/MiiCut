#[macro_use]
mod macros;

pub mod app;
pub mod canvas;
pub mod clipboard;
pub mod cnc_link;
pub mod dimensions;
pub mod dom;
pub mod gcode;
pub mod handlers;
pub mod import_export;
pub mod inputs;
pub mod math;
pub mod prefab;
pub mod render;
pub mod shape;
pub mod shapes;
pub mod status;
pub mod types;
pub mod undoredo;

use crate::app::create_app_vars;
use web_sys::window;

fn main() {
    console_error_panic_hook::set_once();
    let window = window().expect("no global `window` exists");
    create_app_vars(window).expect("Could not access the document");
}
