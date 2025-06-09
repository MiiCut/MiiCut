macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into())
    }
}
macro_rules! init_element {
    ($doc:expr, $id:expr, $ty:ty) => {{
        $doc.get_element_by_id($id)
            .expect(concat!("missing ", $id))
            .dyn_into::<$ty>()?
    }};
}
pub mod canvas;
pub mod clipboard;
pub mod curves;
pub mod dimensions;
pub mod dom;
pub mod inputs;
pub mod math;
pub mod nodes;
pub mod prefab;
pub mod shapes;
pub mod types;
pub mod undoredo;
use crate::dom::*;
use crate::math::*;
use crate::types::EUId;
use crate::types::VUId;
use canvas::{
    CanvasKind, CanvasText, CanvasTextConfig, Canvases, Color, Pattern, TextAlign, TextPos,
};
use clipboard::*;
use dimensions::dim_hv;
use dimensions::dim_linear;
use dimensions::dim_radius;
use inputs::Keys;
use inputs::SystemMouse;
use inputs::*;
use kurbo::{Size, Vec2};
use nodes::DataSet;
use prefab::get_shapes_colors;
use prefab::get_vertices_colors;
use prefab::point_path;
use shapes::drawable::Drawable;
use shapes::drawable::{ClosedShapeType, ClosedShapes};
use std::collections::HashSet;
use std::str::FromStr;
use std::{cell::RefCell, rc::Rc};
use svg::node::element::path::Data;
use svg::read;
use types::Binding;
use types::Couple;
use types::SegBundle;
use undoredo::UndoRedo;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{
    window, Document, Element, Event, FileReader, HtmlCanvasElement, HtmlElement, HtmlInputElement,
    KeyboardEvent, MouseEvent, WheelEvent, Window,
};

type RefAV<E> = Rc<RefCell<AppVars<E>>>;

fn main() {
    console_error_panic_hook::set_once();
    let window = window().expect("no global `window` exists");
    create_app_vars::<ClosedShapes>(window).expect("Could not access the document");
}

#[allow(dead_code)]
struct AppVars<E: Drawable> {
    element_on_creation: Option<E>,
    dataset: DataSet<E>,
    clipboard: Clipboard<E>,
    undo_redo: UndoRedo<E>,

    // DOM
    window: Window,
    document: Document,
    top_menu: HtmlElement,
    left_panel: HtmlElement,
    canvases: Canvases,
    user_icons: HashSet<Icons>,
    tooltip: HtmlElement,
    icon_selected: Icons,

    // Inputs
    user_ui: UserUI,
    pointer_on_canvas: bool,
}
impl<E: Drawable> AppVars<E> {
    fn esc_pressed(&mut self) {
        self.element_on_creation = None;
        self.go_to_arrow_tool();
    }
    fn ctrl_c_pressed(&mut self) {
        if let Icons::Arrow = self.icon_selected.clone() {}
    }
    fn ctrl_v_pressed(&mut self) {}
    fn del_back_pressed(&mut self) {
        if let Icons::Arrow = self.icon_selected {
            self.dataset.delete_selected_elements();
        }
    }
    fn s_pressed(&mut self) {
        self.user_ui.snap.next_linear();
    }
    fn a_pressed(&mut self) {
        self.user_ui.snap.next_angle();
    }
    fn one_pressed(&mut self) {}
    fn t_pressed(&mut self) {
        // if let None = self.on_creation {}
    }
    fn tab_pressed(&mut self) {
        if let Icons::Arrow = self.icon_selected.clone() {
            for id in self.dataset.set_selected.clone().iter() {
                if let Some(node) = self.dataset.get_element_mut(*id) {
                    node.op_next();
                }
            }
        }
    }
    fn undo(&mut self) {
        // // Temporarily take ownership of `undo_redo`
        // let mut undo_redo = std::mem::take(&mut self.undo_redo);
        // // Perform undo operation
        // undo_redo.undo(&mut self.tree);
        // // Put `undo_redo` back into `avb`
        // self.undo_redo = undo_redo;
    }
    fn redo(&mut self) {
        // // Temporarily take ownership of `undo_redo`
        // let mut undo_redo = std::mem::take(&mut self.undo_redo);
        // // Perform redo operation
        // undo_redo.redo(&mut self.tree);
        // // Put `undo_redo` back into `avb`
        // self.undo_redo = undo_redo;
    }

    fn go_to_arrow_tool(&mut self) {
        self.icon_selected = Icons::Arrow;
        self.user_icons
            .iter()
            .for_each(|icon| self.html_deselect_icons(*icon));
        self.html_select_icon(Icons::Arrow);
    }
    fn html_select_icon(&self, icon: Icons) {
        if let Some(html_element) = icon.get_html_element() {
            html_element
                .set_attribute("class", "icon icon-selected")
                .expect("Failed to set class attribute");
        }
    }
    fn html_deselect_icons(&self, icon: Icons) {
        if let Some(html_element) = icon.get_html_element() {
            html_element
                .set_attribute("class", "icon")
                .expect("Failed to set class attribute");
        }
    }
    fn update_inputs(&mut self, mouse_event: MouseEvent, sys_mouse: SystemMouse) -> UserAction {
        self.user_ui.update(
            get_element_width(&self.left_panel),
            get_element_height(&self.top_menu),
            self.canvases.get_drawing_offset(),
            self.canvases.get_drawing_scale(),
            &mouse_event,
            sys_mouse,
        )
    }
    fn set_element_select_vertex(&mut self) -> bool {
        self.dataset.element_select_vertex(&self.user_ui)
    }
    fn set_select_elements(&mut self) {
        self.dataset.select_elements(&self.user_ui);
    }
    fn set_element_highlight_vertex(&mut self) -> bool {
        self.dataset.element_highlight_vertex(&self.user_ui)
    }
    fn set_highlight_elements(&mut self) {
        self.dataset.highlight_elements(&self.user_ui);
    }
    fn set_move_elements(&mut self) -> bool {
        self.dataset.move_elements(&self.user_ui)
    }
    fn set_move_vertex_selected(&mut self) -> bool {
        // Move selected vertex if no elements are moved
        let mut moved = false;
        if let Some((eid, vid)) = self.dataset.elem_vertex_selected {
            // Move the selected vertex
            self.dataset.get_element_mut(eid).map(|e| {
                moved = e.move_vertex(vid, &self.user_ui);
            });
        }
        moved
    }
    fn move_element_on_creation(&mut self) {
        if let Some(elem) = &mut self.element_on_creation {
            let delta = self.user_ui.pointer.curr - self.user_ui.pointer.saved;
            elem.move_drawable(delta);
        }
    }
    fn move_canvas(&mut self) {
        self.canvases
            .move_drawing_offset(self.user_ui.pointer.curr - self.user_ui.pointer.saved);
    }
}

///////////////
// Initialization
fn create_app_vars<E: Drawable>(window: Window) -> Result<(), JsValue> {
    log!("Creating application variables");
    let document = window.document().expect("should have a document on window");
    let c_draw: HtmlCanvasElement = init_element!(document, "mainCanvas", HtmlCanvasElement);
    let c_grid: HtmlCanvasElement = init_element!(document, "gridCanvas", HtmlCanvasElement);
    let c_back: HtmlCanvasElement = init_element!(document, "backgroundCanvas", HtmlCanvasElement);
    let tooltip: HtmlElement = init_element!(document, "tooltip", HtmlElement);
    let left_panel: HtmlElement = init_element!(document, "left-panel", HtmlElement);
    let top_menu: HtmlElement = init_element!(document, "top-menu", HtmlElement);
    let canvases = Canvases::new(c_back, c_grid, c_draw, Size::new(3000., 1500.))?;

    let mut user_icons: HashSet<Icons> = HashSet::new();
    use Icons::*;
    user_icons.insert(Arrow);
    user_icons.insert(Disc);
    user_icons.insert(Triangle);
    user_icons.insert(Square);
    user_icons.insert(Oblong);
    user_icons.insert(PolyRectangle);
    user_icons.insert(Polygon);

    let set = DataSet::<E>::new();
    // let mut creation_set: Vec<Elem<E>> = vec![];
    // let bl = Vec2::new(-100., 10.);
    // let tr = Vec2::new(10., -100.);

    // let center = Vec2::new(0., 40.);
    // let radius = Vec2::new(10., 120.);
    // let sh1 = E::new(ClosedShapeType::Square, &[bl, tr]);

    // let sh3 = E::new(ClosedShapeType::Disc, &[center, radius]);

    // let e1 = Vec2::new(30., -100.);
    // let e2 = Vec2::new(50., -100.);
    // let e3 = Vec2::new(50., 100.);
    // let e4 = Vec2::new(-50., 100.);
    // let e5 = Vec2::new(-50., -70.);
    // let e6 = Vec2::new(-150., -70.);
    // let e7 = Vec2::new(-150., -200.);
    // let e8 = Vec2::new(30., -200.);
    // let sh4 = E::new(
    //     ClosedShapeType::PolyRectangle,
    //     &[e1, e2, e3, e4, e5, e6, e7, e8],
    // );

    // let e1 = Vec2::new(30.3, 50.);
    // let e2 = Vec2::new(100.2, 100.);
    // let e3 = Vec2::new(200.7, 100.);
    // let e4 = Vec2::new(150., 200.3);
    // let e5 = Vec2::new(100., 159.4);
    // let sh5 = E::new(ClosedShapeType::Polygon, &[e1, e2, e3, e4, e5]);

    // if let Some(sh) = sh1 {
    //     creation_set.push(Elem::<E>::new_element("rect", sh));
    // }
    // if let Some(sh) = sh2 {
    //     creation_set.push(Elem::<E>::new_element("oblong", sh));
    // }
    // if let Some(sh) = sh3 {
    //     creation_set.push(Elem::<E>::new_element("disc", sh));
    // }
    // if let Some(sh) = sh4 {
    //     creation_set.push(Elem::<E>::new_element("polyrect", sh));
    // }
    // if let Some(sh) = sh5 {
    //     creation_set.push(Elem::<E>::new_element("polygon", sh));
    // }

    let app_vars = Rc::new(RefCell::new(AppVars {
        element_on_creation: None,
        dataset: set,
        clipboard: Clipboard::new(),
        undo_redo: UndoRedo::new(),
        window,
        document,
        top_menu,
        left_panel,
        canvases,
        //
        user_icons,
        tooltip,
        icon_selected: Icons::Arrow,
        user_ui: UserUI::new(),
        pointer_on_canvas: false,
    }));

    init_menu(app_vars.clone())?;
    // init_context_menu(app_vars.clone())?;
    init_icons(app_vars.clone())?;
    init_status(app_vars.clone())?;
    init_canvas(app_vars.clone())?;
    init_window(app_vars.clone())?;
    resize_canvases(app_vars.clone());

    let av = app_vars;

    reset_origin(av.clone());
    draw_grid_and_rules(av.clone());
    render_drawing(av);

    Ok(())
}

fn init_window<E: Drawable>(av: RefAV<E>) -> Result<(), JsValue> {
    // Resize event
    let pa_cloneprim = av.clone();
    let pa_cloned2 = av.clone();

    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_window_resize(pa_cloneprim.clone(), event);
    });

    pa_cloned2
        .borrow_mut()
        .window
        .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())?;

    closure.forget();

    // Click event
    let pa_cloneprim = av.clone();
    let pa_cloned2 = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_window_click(pa_cloneprim.clone(), event);
    });
    pa_cloned2
        .borrow_mut()
        .window
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
    closure.forget();

    // Key keydown event
    let pa_cloneprim = av.clone();
    let pa_cloned2 = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_window_keydown(pa_cloneprim.clone(), event);
    });
    pa_cloned2
        .borrow_mut()
        .window
        .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())?;
    closure.forget();

    // Key keydup event
    let pa_cloneprim = av.clone();
    let pa_cloned2 = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_window_keyup(pa_cloneprim.clone(), event);
    });
    pa_cloned2
        .borrow_mut()
        .window
        .add_event_listener_with_callback("keyup", closure.as_ref().unchecked_ref())?;
    closure.forget();

    Ok(())
}
fn init_icons<E: Drawable>(av: RefAV<E>) -> Result<(), JsValue> {
    let avb = av.borrow_mut();
    for icon in avb.user_icons.iter() {
        let element_to_set = icon.get_element();
        set_callback(
            av.clone(),
            "click".into(),
            &element_to_set.as_ref().unwrap(),
            Box::new(on_icon_click),
        )?;
        set_callback(
            av.clone(),
            "mouseover".into(),
            &element_to_set.as_ref().unwrap(),
            Box::new(on_icon_mouseover),
        )?;
        set_callback(
            av.clone(),
            "mouseout".into(),
            &element_to_set.as_ref().unwrap(),
            Box::new(on_icon_mouseout),
        )?;
    }

    Ok(())
}
fn init_canvas<E: Drawable>(av: RefAV<E>) -> Result<(), JsValue> {
    log!("Initializing canvas");
    let pam = av.borrow_mut();
    let c_draw = pam.canvases.get_main_canvas();
    set_callback(
        av.clone(),
        "mousedown".into(),
        c_draw,
        Box::new(on_mouse_down),
    )?;
    set_callback(
        av.clone(),
        "contextmenu".into(),
        c_draw,
        Box::new(on_context_menu),
    )?;
    set_callback(
        av.clone(),
        "mousemove".into(),
        c_draw,
        Box::new(on_mouse_move),
    )?;
    set_callback(av.clone(), "mouseup".into(), c_draw, Box::new(on_mouse_up))?;
    set_callback(
        av.clone(),
        "mouseenter".into(),
        c_draw,
        Box::new(on_mouse_enter),
    )?;
    set_callback(
        av.clone(),
        "mouseleave".into(),
        c_draw,
        Box::new(on_mouse_leave),
    )?;
    set_callback(av.clone(), "wheel".into(), c_draw, Box::new(on_mouse_wheel))?;
    Ok(())
}
fn init_menu<E: Drawable>(av: RefAV<E>) -> Result<(), JsValue> {
    let avb = av.borrow_mut();
    let document = avb.document.clone();

    let load_element = document.get_element_by_id("load-option").unwrap();
    let load_element: HtmlElement = load_element.dyn_into::<HtmlElement>()?;

    let file_input = document.get_element_by_id("file-input").unwrap();
    let file_input: HtmlInputElement = file_input.dyn_into::<HtmlInputElement>()?;

    let file_input_clone = file_input.clone();

    let on_load = Closure::wrap(Box::new(move || {
        file_input_clone.click();
    }) as Box<dyn FnMut()>);

    load_element.add_event_listener_with_callback("click", on_load.as_ref().unchecked_ref())?;
    on_load.forget();

    drop(avb);

    let on_file_select = Closure::wrap(Box::new(move || {
        let av_clone = av.clone();
        if let Some(file_input) = document.get_element_by_id("file-input") {
            let file_input: HtmlInputElement = file_input.dyn_into().unwrap();
            let files = file_input.files().unwrap();
            if let Some(file) = files.get(0) {
                let file_reader = FileReader::new().unwrap();

                let on_load = Closure::wrap(Box::new(move |event: web_sys::Event| {
                    let target = event.target().unwrap();
                    let file_reader: FileReader = target.dyn_into().unwrap();
                    if let Some(result) = file_reader.result().unwrap().as_string() {
                        log!("File content loaded!");
                        // Convert the SVG content to your shapes
                        convert_svg_to_shapes(av_clone.clone(), result);
                    }
                }) as Box<dyn FnMut(_)>);

                file_reader
                    .add_event_listener_with_callback("load", on_load.as_ref().unchecked_ref())
                    .unwrap();
                on_load.forget();

                file_reader.read_as_text(&file).unwrap();
            }
        }
    }) as Box<dyn FnMut()>);

    file_input
        .add_event_listener_with_callback("change", on_file_select.as_ref().unchecked_ref())?;
    on_file_select.forget();

    Ok(())
}

fn init_status<E: Drawable>(av: RefAV<E>) -> Result<(), JsValue> {
    let pam = av.borrow_mut();
    let _document = pam.document.clone();

    Ok(())
}
fn set_callback<E: Drawable>(
    av: RefAV<E>,
    event_str: String,
    element: &Element,
    callbacka: Box<dyn Fn(RefAV<E>, Event) + 'static>,
) -> Result<(), JsValue> {
    let event_str_cloned = event_str.clone();

    let cb = Box::new(move |av: RefAV<E>, e: Event| {
        if let Ok(mouse_event) = e.clone().dyn_into::<MouseEvent>() {
            if mouse_event.type_().as_str() == event_str_cloned {
                callbacka(av.clone(), e);
            }
        } else {
            if let Ok(keyboard_event) = e.clone().dyn_into::<KeyboardEvent>() {
                if keyboard_event.type_().as_str() == event_str_cloned {
                    callbacka(av.clone(), e);
                }
            }
        }
    });

    let closure = Closure::wrap(Box::new(move |event: Event| {
        cb(av.clone(), event);
    }) as Box<dyn FnMut(Event)>);

    element
        .add_event_listener_with_callback(&event_str, closure.as_ref().unchecked_ref())
        .map_err(|e| JsValue::from_str(&format!("Failed to add event listener: {:?}", e)))?;

    closure.forget();

    Ok(())
}
fn convert_svg_to_shapes<E: Drawable>(_av: RefAV<E>, svg_data: String) {
    // Process the SVG content using svg::read
    let mut content = svg_data;
    for event in read(&mut content).unwrap() {
        match event {
            svg::parser::Event::Tag(svg::node::element::tag::Path, _, attributes) => {
                if let Some(data) = attributes.get("d") {
                    let data = Data::parse(data).unwrap();
                    for command in data.iter() {
                        match command {
                            svg::node::element::path::Command::Move(..) => {
                                log!("Move command: {:?}", command);
                            }
                            svg::node::element::path::Command::Line(..) => {
                                log!("Line command: {:?}", command);
                            }
                            svg::node::element::path::Command::QuadraticCurve(..) => {
                                log!("Quad command: {:?}", command);
                            }
                            svg::node::element::path::Command::CubicCurve(..) => {
                                log!("Cubic command: {:?}", command);
                            }
                            _ => log!("Unknown command: {:?}", command),
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

#[allow(unused_variables)]
fn update<E: Drawable>(av: RefAV<E>, user_action: UserAction) -> Result<(), MyError> {
    let mut avb = av.borrow_mut();

    match avb.icon_selected {
        Icons::Arrow => match user_action {
            UserAction::ClickDown(button) => {
                if button == MouseButton::Left {
                    avb.canvases.save_drawing_offset();
                    avb.dataset.save_elements_positions();
                    if !avb.set_element_select_vertex() {
                        avb.set_select_elements();
                    }
                }
            }
            UserAction::Move(btn, btn_level) => {
                if btn_level == ButtonLevel::Up {
                    if !avb.set_element_highlight_vertex() {
                        avb.set_highlight_elements();
                    }
                } else {
                    // Elements and vertices selection are mutual exclusive
                    // Moving is done on selected objects
                    // Move Elements
                    match btn {
                        MouseButton::Left => {
                            if !avb.set_move_elements() {
                                // If no elements moved, move vertices
                                avb.set_move_vertex_selected();
                            }
                        }
                        MouseButton::Right => {
                            avb.move_canvas();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        },
        Icons::Disc
        | Icons::Triangle
        | Icons::Square
        | Icons::Oblong
        | Icons::PolyRectangle
        | Icons::Polygon => match user_action {
            UserAction::ClickDown(button) => {
                if button == MouseButton::Left {
                    if let Some(e) = avb.element_on_creation.as_mut() {
                        e.save_vertices_positions();
                        let e_cloned = e.clone();
                        avb.dataset.push_element(e_cloned);
                    }
                } else {
                    if button == MouseButton::Right {
                        // Right click to cancel the creation
                        avb.element_on_creation = None;
                        avb.go_to_arrow_tool();
                    }
                }
            }
            UserAction::Move(_, btn_level) => {
                if btn_level == ButtonLevel::Up {
                    avb.move_element_on_creation();
                }
            }
            _ => {}
        },
    }
    Ok(())
}

fn on_mouse_move<E: Drawable>(av: RefAV<E>, event: Event) {
    if let Ok(mouse_event) = event.clone().dyn_into::<MouseEvent>() {
        let ua = av
            .borrow_mut()
            .update_inputs(mouse_event, SystemMouse::Move);
        if let Some(e) = update(av.clone(), ua).err() {
            log!("ERROR: {}", e);
        }
    }
    render_drawing(av);
}
fn on_mouse_down<E: Drawable>(av: RefAV<E>, event: Event) {
    if let Ok(mouse_event) = event.clone().dyn_into::<MouseEvent>() {
        let ua = av
            .borrow_mut()
            .update_inputs(mouse_event, SystemMouse::Down);
        // Hide the context menu when clicking elsewhere
        display_html_element(DOMElements::ContextMenuShape, false);
        if let Some(e) = update(av.clone(), ua).err() {
            log!("ERROR: {}", e);
        }
    }
    // update_status_bar(&mut pam);
    render_drawing(av);
}
fn on_mouse_up<E: Drawable>(av: RefAV<E>, event: Event) {
    if let Ok(mouse_event) = event.clone().dyn_into::<MouseEvent>() {
        let ua = av.borrow_mut().update_inputs(mouse_event, SystemMouse::Up);
        if let Some(e) = update(av.clone(), ua).err() {
            log!("ERROR: {}", e);
        }
    }
    render_drawing(av);
}
fn on_mouse_wheel<E: Drawable>(av: RefAV<E>, event: Event) {
    if let Ok(wheel_event) = event.dyn_into::<WheelEvent>() {
        wheel_event.prevent_default();
        let mut avb = av.borrow_mut();

        let zoom_factor = 0.04;
        let old_draw_scale = avb.canvases.get_drawing_scale();
        let old_draw_offset = avb.canvases.get_drawing_offset();

        let new_scale = if wheel_event.delta_y() < 0. {
            (old_draw_scale * (1.0 + zoom_factor)).min(10.0) // Zoom in
        } else {
            (old_draw_scale / (1.0 + zoom_factor)).max(0.5) // Zoom out
        };
        avb.canvases.set_drawing_scale(new_scale);

        let canvas_pos = avb.user_ui.canvas_pos;
        let old_draw_offset_rel = canvas_pos - old_draw_offset;
        let new_draw_offset = canvas_pos - old_draw_offset_rel * (new_scale / old_draw_scale);

        avb.canvases.set_drawing_offset(new_draw_offset);

        drop(avb);
        draw_grid_and_rules(av.clone());
        render_drawing(av);
    }
}
fn on_mouse_enter<E: Drawable>(av: RefAV<E>, _event: Event) {
    let mut avb = av.borrow_mut();
    avb.pointer_on_canvas = true;
    let curr_pos = avb.user_ui.pointer.curr;
    avb.user_ui.pointer.saved = curr_pos;
    if let Some(elem) = &mut avb.element_on_creation {
        elem.move_drawable(curr_pos);
        elem.save_vertices_positions();
    }
    drop(avb);
    render_drawing(av);
}
fn on_mouse_leave<E: Drawable>(av: RefAV<E>, _event: Event) {
    let mut avb = av.borrow_mut();
    avb.pointer_on_canvas = false;
    avb.element_on_creation = None;
    avb.go_to_arrow_tool();
    drop(avb);
    render_drawing(av);
}

fn on_context_menu<E: Drawable>(_av: RefAV<E>, event: Event) {
    // Prevent the default context menu from appearing
    event.prevent_default();
}
// fn show_context_menu(_avb: &mut RefMut<'_, AppVars>) {}
// fn _show_contex_menu_shape(pos: Vec2) {
//     // Display the context menu container
//     display_html_element(DOMElements::ContextMenuShape, true);
//     set_pos_html_element(DOMElements::ContextMenuShape, pos);
//     if let Some(_cm_shape) = DOMElements::ContextMenuShape.get_html_element() {
//         // Add a new context menu item
//         // add_menu_item_with_listener(&cm_shape, "cm-shape-new-item", "New Menu Item", || {
//         //     log!("New Menu Item clicked");
//         // });
//     }
// }
// /// Adds a new menu item to the context menu and attaches a click event listener
// fn _add_menu_item_with_listener<F>(menu: &web_sys::Element, id: &str, text: &str, callback: F)
// where
//     F: Fn() + 'static,
// {
//     let document = document();
//     // Create a new anchor element
//     let new_item = document.create_element("a").unwrap();
//     new_item.set_attribute("href", "#").unwrap();
//     new_item.set_id(id);
//     new_item.set_inner_html(text);
//     // Append the new item to the menu
//     menu.append_child(&new_item).unwrap();
//     // Add a click listener to the new item
//     add_click_listener(&new_item, callback);
// }

///////////////
// Window events
fn resize_canvases<E: Drawable>(av: RefAV<E>) {
    log!("resize--called");
    let mut avb = av.borrow_mut();
    let window_width = avb
        .window
        .inner_width()
        .expect("Failed to get window width")
        .as_f64()
        .expect("Width is not a number") as u32;
    let window_height = avb
        .window
        .inner_height()
        .expect("Failed to get window height")
        .as_f64()
        .expect("Height is not a number") as u32;
    let offset_start_x = get_element_width(&avb.left_panel);
    let offset_start_y = get_element_height(&avb.top_menu);
    let width = window_width - offset_start_x;
    let height = window_height - offset_start_y;
    // log!("resize sizes: ({},{})", width, height);
    avb.canvases.resize_canvases(width, height);

    drop(avb);
    draw_grid_and_rules(av.clone());
    render_drawing(av);
}
fn on_window_resize<E: Drawable>(av: RefAV<E>, _event: Event) {
    resize_canvases(av.clone());
    render_drawing(av);
}
fn on_window_click<E: Drawable>(_pa: RefAV<E>, _event: Event) {}
fn on_window_keydown<E: Drawable>(av: RefAV<E>, event: Event) {
    event.prevent_default();
    if let Ok(keyboard_event) = event.dyn_into::<KeyboardEvent>() {
        if let Some(key) = Keys::from_str(&keyboard_event.key()).ok() {
            let mut avb = av.borrow_mut();
            use Keys::*;
            match key {
                Control | Meta => {
                    log!("control pressed");
                    avb.user_ui.keys_states.ctrl_cmd_pressed = true;
                }
                Shift => {
                    log!("shift pressed");
                    avb.user_ui.keys_states.shift_pressed = true;
                }
                Alt => {
                    log!("alt pressed");
                    avb.user_ui.keys_states.alt_pressed = true;
                }
                Delete | Backspace => avb.del_back_pressed(),
                Escape => avb.esc_pressed(),
                // Copy and paste
                CLower => {
                    if avb.user_ui.keys_states.ctrl_cmd_pressed {
                        log!("ctrl-c pressed");
                        avb.ctrl_c_pressed();
                    }
                }
                VLower => {
                    if avb.user_ui.keys_states.ctrl_cmd_pressed {
                        log!("ctrl-v pressed");
                        avb.ctrl_v_pressed();
                    }
                }
                // Bind
                One | Ampersand => {
                    avb.one_pressed();
                }
                // Undo and Redo
                ZLower => {
                    if avb.user_ui.keys_states.ctrl_cmd_pressed {
                        log!("ctrl-z pressed");
                        avb.undo();
                    }
                }
                ZUpper | YLower => {
                    if avb.user_ui.keys_states.ctrl_cmd_pressed {
                        log!("ctrl-Z pressed or ctrl-y pressed");
                        avb.redo();
                    }
                }
                // Entities values snapping
                SLower | SUpper => {
                    avb.s_pressed();
                }
                ALower | AUpper => {
                    avb.a_pressed();
                }
                // Toggle boolean operation (add, substract, intersect)
                TLower => avb.t_pressed(),
                // Change ShapeCustom: A) edge (line, arc,...) or B) vertex (none, fillet, chamfer)
                Tab => avb.tab_pressed(),
                Space => {
                    log!("space pressed");
                }
                _ => (),
            };
            drop(avb);
            render_drawing(av);
        }
    }
}
fn on_window_keyup<E: Drawable>(av: RefAV<E>, event: Event) {
    if let Ok(keyboard_event) = event.dyn_into::<KeyboardEvent>() {
        if let Some(key) = Keys::from_str(&keyboard_event.key()).ok() {
            let mut avb = av.borrow_mut();
            use Keys::*;
            match key {
                Control | Meta => {
                    log!("control released");
                    avb.user_ui.keys_states.ctrl_cmd_pressed = false;
                }
                Shift => {
                    log!("shift released");
                    avb.user_ui.keys_states.shift_pressed = false;
                }
                Alt => {
                    log!("alt released");
                    avb.user_ui.keys_states.alt_pressed = false;
                }
                _ => (),
            }
        }
    }
}
///////////////
// Icons events
fn on_icon_click<E: Drawable>(av: RefAV<E>, event: Event) {
    let mut avb = av.borrow_mut();
    if let Some(target) = event.target() {
        if let Some(element) = wasm_bindgen::JsCast::dyn_ref::<Element>(&target) {
            if let Some(id) = element.get_attribute("id") {
                if let Some(icon) = avb.user_icons.iter().find(|&&k| k.id() == id).cloned() {
                    avb.icon_selected = icon;
                    avb.dataset.set_highlighted.clear();
                    avb.dataset.set_selected.clear();
                    avb.user_icons
                        .iter()
                        .for_each(|icon| avb.html_deselect_icons(*icon));
                    avb.html_select_icon(icon);
                    match avb.icon_selected {
                        Icons::Arrow => {
                            avb.element_on_creation = None;
                        }
                        Icons::Disc => {
                            let sh =
                                E::new(ClosedShapeType::Disc, &[Vec2::ZERO, Vec2::new(100., 0.)]);
                            if let Some(sh) = sh {
                                avb.element_on_creation = Some(sh);
                            }
                        }
                        Icons::Triangle => {
                            let sh = E::new(
                                ClosedShapeType::Triangle,
                                &[Vec2::ZERO, Vec2::new(100., -150.), Vec2::new(200., 0.)],
                            );
                            if let Some(sh) = sh {
                                avb.element_on_creation = Some(sh);
                            }
                        }
                        Icons::Square => {
                            let sh = E::new(
                                ClosedShapeType::Square,
                                &[Vec2::ZERO, Vec2::new(100., -100.)],
                            );
                            if let Some(sh) = sh {
                                avb.element_on_creation = Some(sh);
                            }
                        }
                        Icons::Oblong => {
                            let sh = E::new(
                                ClosedShapeType::Oblong,
                                &[Vec2::ZERO, Vec2::new(100., -100.), Vec2::new(200., 0.)],
                            );
                            if let Some(sh) = sh {
                                avb.element_on_creation = Some(sh);
                            }
                        }
                        Icons::PolyRectangle => {
                            let sh = E::new(
                                ClosedShapeType::PolyRectangle,
                                &[
                                    Vec2::ZERO,
                                    Vec2::new(100., 0.),
                                    Vec2::new(100., 100.),
                                    Vec2::new(50., 100.),
                                    Vec2::new(50., 150.),
                                    Vec2::new(0., 150.),
                                ],
                            );
                            if let Some(sh) = sh {
                                avb.element_on_creation = Some(sh);
                            }
                        }
                        Icons::Polygon => {
                            let sh = E::new(
                                ClosedShapeType::Polygon,
                                &[
                                    Vec2::ZERO,
                                    Vec2::new(100., -100.),
                                    Vec2::new(200., 70.),
                                    Vec2::new(80., 0.),
                                    Vec2::new(30., 50.),
                                ],
                            );
                            if let Some(sh) = sh {
                                avb.element_on_creation = Some(sh);
                            }
                        }
                    }
                }
            }
        }
    }
    drop(avb);
    render_drawing(av);
}
fn on_icon_mouseover<E: Drawable>(av: RefAV<E>, event: Event) {
    let avb = av.borrow_mut();
    if let Some(target) = event.target() {
        if let Some(element) = wasm_bindgen::JsCast::dyn_ref::<Element>(&target) {
            if let Some(data_tooltip) = element.get_attribute("data-tooltip") {
                let tooltip_html = &avb.tooltip;
                tooltip_html.set_inner_text(&data_tooltip);
                tooltip_html
                    .style()
                    .set_property("display", "block")
                    .unwrap();
                if let Some(mouse_event) = event.dyn_ref::<MouseEvent>() {
                    let x = mouse_event.page_x();
                    let y = mouse_event.page_y();
                    tooltip_html
                        .style()
                        .set_property("left", &format!("{}px", x + 10))
                        .unwrap();
                    tooltip_html
                        .style()
                        .set_property("top", &format!("{}px", y + 10))
                        .unwrap();
                }
            }
        }
    }
}
fn on_icon_mouseout<E: Drawable>(av: RefAV<E>, _event: Event) {
    av.borrow_mut()
        .tooltip
        .style()
        .set_property("display", "none")
        .expect("Failed to set display property");
}

///////////////
// Rendering
fn reset_origin<E: Drawable>(av: RefAV<E>) {
    av.borrow_mut().canvases.reset_origin();
}
fn draw_grid_and_rules<E: Drawable>(av: RefAV<E>) {
    av.borrow_mut().canvases.draw_origin();
}
fn render_drawing<E: Drawable>(av: RefAV<E>) {
    let mut avb = av.borrow_mut();
    // use IconsShapes::*;
    // Get the Performance API
    // let performance = window().unwrap().performance().unwrap();
    // let start_time = performance.now();

    // avb.set.recalc_full_segs();

    // let scale = avb.canvases.get_drawing_scale();
    avb.canvases.clear_main_canvas();
    // let das = &avb.canvases.get_drawing_size();
    // let cinfo = avb.canvases.get_canvas_infos();
    // Draw the final contour
    // let full_segs = avb.set.shapes.get_full_segs();
    // avb.canvases.draw_closed_path(
    //     &CanvasKind::Draw,
    //     full_segs,
    //     Pattern::Composed(true),
    //     get_final_contour_colors(Status::default()).color,
    //     get_final_contour_colors(Status::default()).fill_color,
    //     vec![],
    // );

    // Draw the elements (shapes outlines)
    for (eid, e) in avb.dataset.set.iter() {
        let colors = get_shapes_colors(
            avb.dataset.set_selected.contains(eid),
            avb.dataset.set_highlighted.contains(eid),
        );
        let path = e.get_bezpath();
        avb.canvases.draw_path(
            &CanvasKind::Draw,
            path,
            Pattern::Composed(true),
            colors,
            vec![],
        );
    }
    // Draw the elements vertices, get the bind on the same time
    let mut binds = Binding::<(EUId, VUId)>::new();
    let vid_sel = avb.dataset.elem_vertex_selected.map(|(_, vid)| vid);
    let vid_hig = avb.dataset.elem_vertex_highlighted.map(|(_, vid)| vid);
    for (eid, e) in avb.dataset.set.iter() {
        for (v_id, vertex) in e.get_vertices().iter() {
            let colors = get_vertices_colors(vid_sel == Some(*v_id), vid_hig == Some(*v_id));
            avb.canvases.draw_path(
                &CanvasKind::Draw,
                &point_path(vertex.curr, 1.),
                Pattern::Point,
                colors,
                vec![],
            );
            // If the vertex is bound to other vertices, store the binding
            binds.extend(vertex.bind.iter().map(|(eid2, vid2)| {
                Couple((eid.clone(), v_id.clone()), (eid2.clone(), vid2.clone()))
            }));
        }
        // Dimensions
        use ClosedShapeType::*;
        match e.get_shape_type() {
            Polygon | PolyRectangle | Triangle => {
                for (v1, v2) in e
                    .get_vertices()
                    .iter()
                    .zip(e.get_vertices().iter().cycle().skip(1))
                    .map(|(v1, v2)| (v1.1.curr, v2.1.curr))
                {
                    if let Some(seg) = SegBundle::new(v1, v2) {
                        let (path, pattern, color, text) =
                            dim_hv(seg, avb.canvases.get_canvas_infos());
                        avb.canvases
                            .draw_path(&CanvasKind::Draw, &path, pattern, color, text);
                    }
                }
            }
            Square => {
                for (v1, v2) in e
                    .get_vertices()
                    .iter()
                    .zip(e.get_vertices().iter().cycle().skip(1))
                    .take(2)
                    .map(|(v1, v2)| (v1.1.curr, v2.1.curr))
                {
                    if let Some(seg) = SegBundle::new(v1, v2) {
                        let (path, pattern, color, text) =
                            dim_hv(seg, avb.canvases.get_canvas_infos());
                        avb.canvases
                            .draw_path(&CanvasKind::Draw, &path, pattern, color, text);
                    }
                }
            }
            Oblong => {
                let v: Vec<Vec2> = e.get_vertices().iter().map(|(_, v)| v.curr).collect();
                if v.len() == 3 {
                    if let Some(seg1) = SegBundle::new(v[0], v[2]) {
                        // Draw the main segment
                        let (path, pattern, color, text) =
                            dim_radius(seg1, avb.canvases.get_canvas_infos());
                        avb.canvases
                            .draw_path(&CanvasKind::Draw, &path, pattern, color, text);
                        // Draw the radius segment
                        if let Some(seg2) = SegBundle::new(seg1.m, v[1]) {
                            let (path, pattern, color, text) =
                                dim_linear(seg2, avb.canvases.get_canvas_infos());
                            avb.canvases
                                .draw_path(&CanvasKind::Draw, &path, pattern, color, text);
                        }
                    }
                }
            }
            Disc => {
                let v: Vec<Vec2> = e.get_vertices().iter().map(|(_, v)| v.curr).collect();
                if v.len() == 2 {
                    if let Some(seg) = SegBundle::new(v[0], v[1]) {
                        // Draw the radius segment
                        let (path, pattern, color, text) =
                            dim_radius(seg, avb.canvases.get_canvas_infos());
                        avb.canvases
                            .draw_path(&CanvasKind::Draw, &path, pattern, color, text);
                    }
                }
            }
        }
    }

    // Draw the binds dimensions
    for Couple((eid1, vid1), (eid2, vid2)) in binds.iter() {
        // Get elements e1 and e2
        if let (Some(e1), Some(e2)) = (
            avb.dataset.get_element(*eid1),
            avb.dataset.get_element(*eid2),
        ) {
            // Get vertices v1 and v2
            if let (Some(v1), Some(v2)) = (e1.get_vertex(vid1), e2.get_vertex(vid2)) {
                // Draw the binding segment
                if let Some(seg) = SegBundle::new(v1.curr, v2.curr) {
                    let (path, pattern, color, text) = dim_hv(seg, avb.canvases.get_canvas_infos());
                    avb.canvases
                        .draw_path(&CanvasKind::Draw, &path, pattern, color, text);
                }
            }
        }
    }

    // CLIPBOARD
    // if let Some(item) = avb.clipboard.get_paste() {
    //     match item {}
    // }
    // Draw the data that is being created
    // let create_colors = Colors {
    //     color: Color::Black,
    //     fill_color: Color::Gray90Opacity,
    // };
    // match avb.icon_selected {
    //     Icons::IShapes(ishape) => match ishape {
    //         Disc => {
    //             if let Some(points) = avb.on_creation.clone() {
    //                 let center = points.0.get(0).get_vertex().curr;
    //                 let end = points.1;
    //                 let radius = (end - center).length();
    //                 let tolerance = 0.1;
    //                 avb.canvases.draw_path(
    //                     &CanvasKind::Draw,
    //                     (
    //                         point_path(center, scale),
    //                         Pattern::Point,
    //                         create_colors,
    //                         vec![],
    //                     ),
    //                 );
    //                 avb.canvases.draw_path(
    //                     &CanvasKind::Draw,
    //                     (
    //                         Circle::new(center.to_point(), radius)
    //                             .path_elements(tolerance)
    //                             .collect(),
    //                         Pattern::OnCreation,
    //                         create_colors,
    //                         vec![],
    //                     ),
    //                 );
    //                 SegBundle::new(center, end).and_then(|bdl| {
    //                     avb.canvases
    //                         .draw_path(&CanvasKind::Draw, dim_radius(bdl, cinfo));
    //                     Some(())
    //                 });
    //             }
    //         }
    //         Rectangle | RectangleFillet => {
    //             if let Some(points) = avb.on_creation.clone() {
    //                 let v1 = points.0.get(0).get_vertex().curr;
    //                 let v3 = points.1;
    //                 let v2 = Vec2::new(v1.x, v3.y);
    //                 let v4 = Vec2::new(v3.x, v1.y);
    //                 let mut vs = VecRing::from_element(v1);
    //                 vs.push(v2);
    //                 vs.push(v3);
    //                 vs.push(v4);
    //                 // Points
    //                 for idx in 0..vs.len() as i64 {
    //                     avb.canvases.draw_path(
    //                         &CanvasKind::Draw,
    //                         (
    //                             point_path(*vs.get(idx), scale),
    //                             Pattern::Point,
    //                             create_colors,
    //                             vec![],
    //                         ),
    //                     );
    //                 }
    //                 // Lines
    //                 let bez_path = BezPath::from_vec(vec![
    //                     PathEl::MoveTo(v1.to_point()),
    //                     PathEl::LineTo(v2.to_point()),
    //                     PathEl::LineTo(v3.to_point()),
    //                     PathEl::LineTo(v4.to_point()),
    //                     PathEl::ClosePath,
    //                 ]);
    //                 avb.canvases.draw_path(
    //                     &CanvasKind::Draw,
    //                     (bez_path, Pattern::OnCreation, create_colors, vec![]),
    //                 );
    //                 // Dimensions
    //                 SegBundle::new(v4, v3).and_then(|bdl| {
    //                     avb.canvases
    //                         .draw_path(&CanvasKind::Draw, dim_linear(bdl, cinfo));
    //                     Some(())
    //                 });
    //                 SegBundle::new(v1, v4).and_then(|bdl| {
    //                     avb.canvases
    //                         .draw_path(&CanvasKind::Draw, dim_linear(bdl, cinfo));
    //                     Some(())
    //                 });
    //             }
    //         }
    //         Oblong => {
    //             if let Some(points) = avb.on_creation.clone() {
    //                 let start = points.0.get(0).get_vertex().curr;
    //                 let end = points.1;
    //                 avb.canvases.draw_path(
    //                     &CanvasKind::Draw,
    //                     (
    //                         point_path(start, scale),
    //                         Pattern::Point,
    //                         create_colors,
    //                         vec![],
    //                     ),
    //                 );
    //                 avb.canvases.draw_path(
    //                     &CanvasKind::Draw,
    //                     (
    //                         line_path(start, end),
    //                         Pattern::OnCreation,
    //                         create_colors,
    //                         vec![],
    //                     ),
    //                 );
    //                 SegBundle::new(start, end).and_then(|bdl| {
    //                     avb.canvases
    //                         .draw_path(&CanvasKind::Draw, dim_linear_angle(bdl, cinfo));
    //                     Some(())
    //                 });
    //             }
    //         }
    //         Custom => {
    //             if let Some(points) = avb.on_creation.clone() {
    //                 let vlen = points.0.len() as i64;
    //                 for idx in 0..vlen {
    //                     let vertex = points.0.get(idx).get_vertex().curr;
    //                     let vertex_next = if idx + 1 == vlen {
    //                         points.1
    //                     } else {
    //                         points.0.get(idx + 1).get_vertex().curr
    //                     };
    //                     avb.canvases.draw_path(
    //                         &CanvasKind::Draw,
    //                         (
    //                             point_path(vertex, scale),
    //                             Pattern::Point,
    //                             create_colors,
    //                             vec![],
    //                         ),
    //                     );
    //                     avb.canvases.draw_path(
    //                         &CanvasKind::Draw,
    //                         (
    //                             line_path(vertex, vertex_next),
    //                             Pattern::OnCreation,
    //                             create_colors,
    //                             vec![],
    //                         ),
    //                     );
    //                     SegBundle::new(vertex, vertex_next).and_then(|bdl| {
    //                         avb.canvases
    //                             .draw_path(&CanvasKind::Draw, dim_linear_angle(bdl, cinfo));
    //                         Some(())
    //                     });
    //                 }
    //             }
    //         }
    //     },
    //     _ => (),
    // }
    // let end_time = performance.now();
    // log!("Rendering time: {:.2} ms", end_time - start_time);

    // Only if the pointer is over the canvas
    if avb.pointer_on_canvas {
        // Draw element on creation if any
        if let Some(e) = avb.element_on_creation.as_ref() {
            let colors = get_shapes_colors(false, false);
            let path = e.get_bezpath();
            avb.canvases.draw_path(
                &CanvasKind::Draw,
                path,
                Pattern::Composed(true),
                colors,
                vec![],
            );
            e.get_vertices().iter().for_each(|(_, v)| {
                avb.canvases.draw_path(
                    &CanvasKind::Draw,
                    &point_path(v.curr, 1.),
                    Pattern::Point,
                    get_vertices_colors(false, false),
                    vec![],
                );
            });
        }
        // Draw the pointer
        avb.canvases.draw_pointer(avb.user_ui.pointer.curr);
    }
    drop(avb);
    render_informations(av);
}

fn render_informations<E: Drawable>(av: RefAV<E>) {
    let avb = av.borrow_mut();
    // avb.canvases.clear_background_canvas();
    let c_size = avb.canvases.get_canvas_size();
    let pos = avb.user_ui.pointer.curr;
    avb.canvases.direct_text(
        &CanvasKind::Draw,
        &CanvasText::new(
            format!("({:.2},{:.2})", pos.x, pos.y),
            TextPos::PosCustom(Vec2::new(30., c_size.height - 10.)),
            CanvasTextConfig::new(Color::Rules, 0., TextAlign::Left, 20, 0.4),
        ),
    );
    avb.canvases.direct_text(
        &CanvasKind::Draw,
        &CanvasText::new(
            format!(
                "Snap value (PRESS S): {:.0} mm / {:.0} °",
                avb.user_ui.snap.linear(),
                avb.user_ui.snap.angle()
            ),
            TextPos::PosCustom(Vec2::new(300., c_size.height - 10.)),
            CanvasTextConfig::new(Color::Rules, 0., TextAlign::Left, 20, 0.4),
        ),
    );
}
