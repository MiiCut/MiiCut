mod canvas;
pub mod closed_shapes;
pub mod dom;
mod math;
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

    // let document = window()
    //     .and_then(|win| win.document())
    //     .expect("Could not access the document");
    // let body = document.body().expect("Could not access document.body");
    // let text_node = document.create_text_node("Hello, world from Vanilla Rust!");
    // body.append_child(text_node.as_ref())
    //     .expect("Failed to append text");
}
