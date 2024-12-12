pub mod canvas;
pub mod canvas_core;
pub mod closed_shapes;
pub mod dom;
pub mod math;
pub mod prefab;
pub mod shape_hole;
pub mod shape_oblong;
pub mod shape_rectangle;
pub mod shape_rectangle_rounded;
use canvas::create_playing_area;
use web_sys::window;

fn main() {
    console_error_panic_hook::set_once();
    let window = window().expect("no global `window` exists");
    create_playing_area(window).expect("Could not access the document");
}
