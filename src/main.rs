#[macro_use]
pub mod helpers;
pub mod app;
pub mod canvas;
pub mod clipboard;
pub mod commands;
pub mod dimensions;
pub mod dom;
pub mod examples_gen;
pub mod import_export;
pub mod inputs;
pub mod machine;
pub mod shape;
pub mod shapes;
pub mod status;
pub mod tutorial_gen;
pub mod types;
pub mod ui;
pub mod undoredo;
pub mod view_draw;
pub mod view_gcode;
pub mod view_machine;
pub mod view_play;
pub mod view_toolpath;
pub mod voronoi;

use crate::app::create_app_vars;
use web_sys::window;

fn main() {
    console_error_panic_hook::set_once();
    let window = window().expect("no global `window` exists");
    create_app_vars(window).expect("Could not access the document");
}
