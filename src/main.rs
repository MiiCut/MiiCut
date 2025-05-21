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

use crate::dom::*;
use crate::math::*;
use canvas::{
    CanvasKind, CanvasText, CanvasTextConfig, Canvases, Color, Pattern, TextAlign, TextPos,
};
use clipboard::*;
use curves::half_edge::HalfEdge;
use dimensions::*;
use inputs::Keys;
use inputs::SystemMouse;
use inputs::*;
use kurbo::{BezPath, Circle, PathEl, Shape, Size, Vec2};
use nodes::Set;
use prefab::*;
use shapes::drawable::Drawable;
use shapes::drawable::Shapes;
use std::collections::HashSet;
use std::str::FromStr;
use std::{
    cell::{RefCell, RefMut},
    rc::Rc,
};
use svg::node::element::path::Data;
use svg::read;
use types::*;
use undoredo::UndoRedo;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{
    window, Document, Element, Event, FileReader, HtmlCanvasElement, HtmlElement, HtmlInputElement,
    KeyboardEvent, MouseEvent, WheelEvent, Window,
};

type RefAV<E> = Rc<RefCell<AppVars<E>>>;
type ElementCallback<E> = Box<dyn Fn(RefAV<E>, Event) + 'static>;

fn main() {
    console_error_panic_hook::set_once();
    let window = window().expect("no global `window` exists");
    create_app_vars::<Shapes>(window).expect("Could not access the document");
}

struct AppVars<E: Drawable> {
    set: Set<E>,
    on_creation: Option<(VecRing<HalfEdge>, Vec2)>,

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
    userui: UserUI,
}
impl<E: Drawable> AppVars<E> {
    fn esc_pressed(&mut self) {
        self.on_creation = None;
        self.go_to_arrow_tool();
    }
    fn ctrl_c_pressed(&mut self) {
        if let Icons::Arrow = self.icon_selected.clone() {}
    }
    fn ctrl_v_pressed(&mut self) {}
    fn del_back_pressed(&mut self) {}
    fn s_pressed(&mut self) {
        self.userui.snap.next_linear();
    }
    fn a_pressed(&mut self) {
        self.userui.snap.next_angle();
    }
    fn t_pressed(&mut self) {
        if let None = self.on_creation {}
    }
    fn tab_pressed(&mut self) {
        if let Icons::Arrow = self.icon_selected.clone() {
            for id in self.set.nodes_state.get_selected_nodes().iter() {
                if let Some(node) = self.set.get_mut(*id) {
                    node.element.op_next();
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
    fn update_inputs(&mut self, event: Event, sys_mouse: SystemMouse) {
        let drawing_offset = self.canvases.get_drawing_offset();
        let drawing_scale = self.canvases.get_drawing_scale();
        let offset_start_x = get_element_width(&self.left_panel);
        let offset_start_y = get_element_height(&self.top_menu);
        self.userui.update(
            offset_start_x,
            offset_start_y,
            drawing_offset,
            drawing_scale,
            &event,
            sys_mouse,
        );
        self.userui.magnetized = false;
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
    use IconsShapes::*;
    user_icons.insert(Arrow);
    user_icons.insert(IShapes(Disc));
    user_icons.insert(IShapes(Rectangle));
    user_icons.insert(IShapes(RectangleFillet));
    user_icons.insert(IShapes(Oblong));
    user_icons.insert(IShapes(Custom));

    let app_vars = Rc::new(RefCell::new(AppVars {
        set: Set::<E>::new(),
        on_creation: None,
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
        userui: UserUI::new(),
    }));

    init_menu(app_vars.clone())?;
    // init_context_menu(app_vars.clone())?;
    init_icons(app_vars.clone())?;
    init_status(app_vars.clone())?;
    init_canvas(app_vars.clone())?;
    init_window(app_vars.clone())?;
    resize_canvases(app_vars.clone());

    let av = app_vars.clone();
    let mut avb = av.borrow_mut();

    reset_origin(&mut avb);
    update_informations(&mut avb);
    draw_grid_and_rules(&mut avb);
    render_drawing(&mut avb);

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
                        drop(av_clone.borrow_mut());
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
    callbacka: ElementCallback<E>,
) -> Result<(), JsValue> {
    let event_str_cloned = event_str.clone();

    let callback = Box::new(move |av: RefAV<E>, e: Event| {
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
        callback(av.clone(), event);
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
fn update<E: Drawable>(
    avb: &mut RefMut<'_, AppVars<E>>,
    event: Event,
    sys_mouse: SystemMouse,
) -> Result<(), MyError> {
    avb.update_inputs(event, sys_mouse);
    let icon_selected = avb.icon_selected.clone();
    let snap = avb.userui.snap;
    let inputs = avb.userui;

    Ok(())
}

fn update_informations<E: Drawable>(avb: &mut RefMut<'_, AppVars<E>>) {
    avb.canvases.clear_background_canvas();
    let c_size = avb.canvases.get_canvas_size();
    let pos = avb.userui.pointer.curr;
    let ckind = CanvasKind::Background;
    avb.canvases.direct_text(
        &ckind,
        &CanvasText::new(
            format!("( {:.1} , {:.1} )", pos.x, pos.y),
            TextPos::PosCustom(Vec2::new(c_size.width - 10., c_size.height - 10.)),
            CanvasTextConfig::new(Color::Rules, 0., TextAlign::Right, 20, 0.4),
        ),
    );
    avb.canvases.direct_text(
        &ckind,
        &CanvasText::new(
            format!(
                "Snap value (PRESS S): {:.0} mm / {:.0} °",
                avb.userui.snap.linear(),
                avb.userui.snap.angle()
            ),
            TextPos::Pos1(c_size.height),
            CanvasTextConfig::new(Color::Rules, 0., TextAlign::Left, 20, 0.4),
        ),
    );
}
fn on_mouse_move<E: Drawable>(av: RefAV<E>, event: Event) {
    let mut avb = av.borrow_mut();
    if let Some(e) = update(&mut avb, event, SystemMouse::Move).err() {
        log!("ERROR: {}", e);
    }
    update_informations(&mut avb);
    render_drawing(&mut avb);
    drop(avb);
}
fn on_mouse_down<E: Drawable>(av: RefAV<E>, event: Event) {
    let mut avb = av.borrow_mut();
    // Hide the context menu when clicking elsewhere
    display_html_element(DOMElements::ContextMenuShape, false);
    if let Some(e) = update(&mut avb, event, SystemMouse::Down).err() {
        log!("ERROR: {}", e);
    }
    // update_status_bar(&mut pam);
    render_drawing(&mut avb);
    drop(avb);
}
fn on_mouse_up<E: Drawable>(av: RefAV<E>, event: Event) {
    let mut avb = av.borrow_mut();
    if let Some(e) = update(&mut avb, event, SystemMouse::Up).err() {
        log!("ERROR: {}", e);
    }
    update_informations(&mut avb);
    render_drawing(&mut avb);
    drop(avb);
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

        let canvas_pos = avb.userui.canvas_pos;
        let old_draw_offset_rel = canvas_pos - old_draw_offset;
        let new_draw_offset = canvas_pos - old_draw_offset_rel * (new_scale / old_draw_scale);

        avb.canvases.set_drawing_offset(new_draw_offset);

        draw_grid_and_rules(&mut avb);
        render_drawing(&mut avb);
        drop(avb);
    }
}
fn on_mouse_enter<E: Drawable>(av: RefAV<E>, _event: Event) {
    let mut avb = av.borrow_mut();
    avb.userui.active = true;
    render_drawing(&mut avb);
    drop(avb);
}
fn on_mouse_leave<E: Drawable>(av: RefAV<E>, _event: Event) {
    let mut avb = av.borrow_mut();
    avb.userui.active = false;
    render_drawing(&mut avb);
    drop(avb);
}

fn on_context_menu<E: Drawable>(_av: RefAV<E>, event: Event) {
    // Prevent the default context menu from appearing
    event.prevent_default();
    // let mut avb = av.borrow_mut();
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
    let mut pam = av.borrow_mut();
    let window_width = pam
        .window
        .inner_width()
        .expect("Failed to get window width")
        .as_f64()
        .expect("Width is not a number") as u32;
    let window_height = pam
        .window
        .inner_height()
        .expect("Failed to get window height")
        .as_f64()
        .expect("Height is not a number") as u32;
    let offset_start_x = get_element_width(&pam.left_panel);
    let offset_start_y = get_element_height(&pam.top_menu);
    let width = window_width - offset_start_x;
    let height = window_height - offset_start_y;
    // log!("resize sizes: ({},{})", width, height);
    pam.canvases.resize_canvases(width, height);

    draw_grid_and_rules(&mut pam);
    render_drawing(&mut pam);
    drop(pam);
}
fn on_window_resize<E: Drawable>(av: RefAV<E>, _event: Event) {
    resize_canvases(av.clone());
    let mut pam = av.borrow_mut();
    render_drawing(&mut pam);
    drop(pam);
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
                    avb.userui.keys_states.crtl_cmd_pressed = true;
                }
                Shift => {
                    log!("shift pressed");
                    avb.userui.keys_states.shift_pressed = true;
                }
                Alt => {
                    log!("alt pressed");
                    avb.userui.keys_states.alt_pressed = true;
                }
                Delete | Backspace => avb.del_back_pressed(),
                Escape => avb.esc_pressed(),
                // Copy and paste
                CLower => {
                    if avb.userui.keys_states.crtl_cmd_pressed {
                        log!("ctrl-c pressed");
                        avb.ctrl_c_pressed();
                    }
                }
                VLower => {
                    if avb.userui.keys_states.crtl_cmd_pressed {
                        log!("ctrl-v pressed");
                        avb.ctrl_v_pressed();
                    }
                }
                // Undo and Redo
                ZLower => {
                    if avb.userui.keys_states.crtl_cmd_pressed {
                        log!("ctrl-z pressed");
                        avb.undo();
                    }
                }
                ZUpper | YLower => {
                    if avb.userui.keys_states.crtl_cmd_pressed {
                        log!("ctrl-Z pressed or ctrl-y pressed");
                        avb.redo();
                    }
                }
                // Entities values snapping
                SLower | SUpper => {
                    avb.s_pressed();
                    update_informations(&mut avb);
                }
                ALower | AUpper => {
                    avb.a_pressed();
                    update_informations(&mut avb);
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

            render_drawing(&mut avb);
            drop(avb);
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
                    avb.userui.keys_states.crtl_cmd_pressed = false;
                }
                Shift => {
                    log!("shift released");
                    avb.userui.keys_states.shift_pressed = false;
                }
                Alt => {
                    log!("alt released");
                    avb.userui.keys_states.alt_pressed = false;
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
                    avb.on_creation = None;
                    avb.set.nodes_state.clr_all_highlighted();
                    avb.set.nodes_state.clr_all_selected();
                    avb.user_icons
                        .iter()
                        .for_each(|icon| avb.html_deselect_icons(*icon));
                    avb.html_select_icon(icon);
                }
            }
        }
    }
    render_drawing(&mut avb);
    drop(avb);
}
fn on_icon_mouseover<E: Drawable>(av: RefAV<E>, event: Event) {
    let pam = av.borrow_mut();
    if let Some(target) = event.target() {
        if let Some(element) = wasm_bindgen::JsCast::dyn_ref::<Element>(&target) {
            if let Some(data_tooltip) = element.get_attribute("data-tooltip") {
                let tooltip_html = &pam.tooltip;
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
fn reset_origin<E: Drawable>(avb: &mut RefMut<'_, AppVars<E>>) {
    avb.canvases.reset_origin();
}
fn draw_grid_and_rules<E: Drawable>(avb: &mut RefMut<'_, AppVars<E>>) {
    avb.canvases.draw_origin();
}
fn render_drawing<E: Drawable>(avb: &mut RefMut<'_, AppVars<E>>) {
    use IconsShapes::*;
    use HS::*;
    // Get the Performance API
    // let performance = window().unwrap().performance().unwrap();
    // let start_time = performance.now();
    avb.set.recalc_full_segs();

    let scale = avb.canvases.get_drawing_scale();
    avb.canvases.clear_main_canvas();
    let das = &avb.canvases.get_drawing_size();
    let cinfo = avb.canvases.get_canvas_infos();

    // Draw inputs.pointer
    if avb.userui.active {
        avb.canvases.draw_pointer(avb.userui.pointer.curr);
    }

    // SHAPES: Draw the final contour
    let full_segs = avb.set.shapes.get_full_segs();
    avb.canvases.draw_closed_path(
        &CanvasKind::Draw,
        full_segs,
        Pattern::Composed(true),
        get_final_contour_colors(Status::default()).color,
        get_final_contour_colors(Status::default()).fill_color,
        vec![],
    );
    // SHAPES: Draw the outlines
    for e in avb.set.elements() {
        e.get_kind()
            .get_paths_and_patterns(das, cinfo)
            .iter()
            .for_each(|(path, pattern, colors)| {
                avb.canvases
                    .draw_path(&CanvasKind::Draw, (path.clone(), *pattern, *colors, vec![]));
            });
    }
    // SHAPES: Draw the primitives
    for e in avb.set.elements() {
        e.get_kind()
            .get_prim_paths_and_patterns(das, cinfo)
            .iter()
            .for_each(|(path, pattern, colors)| {
                avb.canvases
                    .draw_path(&CanvasKind::Draw, (path.clone(), *pattern, *colors, vec![]));
            });
    }
    // SHAPES: Draw the controls points
    for e in avb.set.elements() {
        for (path, pattern, colors) in e.get_kind().get_controls_paths_and_patterns(das, cinfo) {
            avb.canvases
                .draw_path(&CanvasKind::Draw, (path, pattern, colors, vec![]));
        }
    }
    // SHAPES: Draw dimensions
    for e in avb.set.elements() {
        if e.get_kind().get_state(IsHS(Select))
            || e.get_kind().get_state(IsHS(Highlight))
            || e.get_kind().get_state(IsAControlHS(Select))
            || e.get_kind().get_state(IsAControlHS(Highlight))
        {
            for bundle in e.get_kind().get_dimensions_paths_and_patterns(das, cinfo) {
                avb.canvases.draw_path(&CanvasKind::Draw, bundle);
            }
        }
    }

    // CLIPBOARD
    if let Some(item) = avb.clipboard.get_paste() {
        match item {}
    }

    // Draw the data that is being created
    let create_colors = get_on_creation_colors(Status::default());
    match avb.icon_selected {
        Icons::IShapes(ishape) => match ishape {
            Disc => {
                if let Some(points) = avb.on_creation.clone() {
                    let center = points.0.get(0).get_vertex().curr;
                    let end = points.1;
                    let radius = (end - center).length();
                    let tolerance = 0.1;

                    avb.canvases.draw_path(
                        &CanvasKind::Draw,
                        (
                            point_path(center, scale),
                            Pattern::Point,
                            create_colors,
                            vec![],
                        ),
                    );
                    avb.canvases.draw_path(
                        &CanvasKind::Draw,
                        (
                            Circle::new(center.to_point(), radius)
                                .path_elements(tolerance)
                                .collect(),
                            Pattern::OnCreation,
                            create_colors,
                            vec![],
                        ),
                    );
                    SegBundle::new(center, end).and_then(|bdl| {
                        avb.canvases.draw_path(
                            &CanvasKind::Draw,
                            dim_radius(bdl, cinfo, Status::default()),
                        );
                        Some(())
                    });
                }
            }
            Rectangle | RectangleFillet => {
                if let Some(points) = avb.on_creation.clone() {
                    let v1 = points.0.get(0).get_vertex().curr;
                    let v3 = points.1;
                    let v2 = Vec2::new(v1.x, v3.y);
                    let v4 = Vec2::new(v3.x, v1.y);
                    let mut vs = VecRing::from_element(v1);
                    vs.push(v2);
                    vs.push(v3);
                    vs.push(v4);
                    // Points
                    for idx in 0..vs.len() as i64 {
                        avb.canvases.draw_path(
                            &CanvasKind::Draw,
                            (
                                point_path(*vs.get(idx), scale),
                                Pattern::Point,
                                create_colors,
                                vec![],
                            ),
                        );
                    }
                    // Lines
                    let bez_path = BezPath::from_vec(vec![
                        PathEl::MoveTo(v1.to_point()),
                        PathEl::LineTo(v2.to_point()),
                        PathEl::LineTo(v3.to_point()),
                        PathEl::LineTo(v4.to_point()),
                        PathEl::ClosePath,
                    ]);
                    avb.canvases.draw_path(
                        &CanvasKind::Draw,
                        (bez_path, Pattern::OnCreation, create_colors, vec![]),
                    );
                    // Dimensions
                    SegBundle::new(v4, v3).and_then(|bdl| {
                        avb.canvases.draw_path(
                            &CanvasKind::Draw,
                            dim_linear(bdl, cinfo, Status::default()),
                        );
                        Some(())
                    });
                    SegBundle::new(v1, v4).and_then(|bdl| {
                        avb.canvases.draw_path(
                            &CanvasKind::Draw,
                            dim_linear(bdl, cinfo, Status::default()),
                        );
                        Some(())
                    });
                }
            }
            Oblong => {
                if let Some(points) = avb.on_creation.clone() {
                    let start = points.0.get(0).get_vertex().curr;
                    let end = points.1;
                    avb.canvases.draw_path(
                        &CanvasKind::Draw,
                        (
                            point_path(start, scale),
                            Pattern::Point,
                            create_colors,
                            vec![],
                        ),
                    );
                    avb.canvases.draw_path(
                        &CanvasKind::Draw,
                        (
                            line_path(start, end),
                            Pattern::OnCreation,
                            create_colors,
                            vec![],
                        ),
                    );
                    SegBundle::new(start, end).and_then(|bdl| {
                        avb.canvases.draw_path(
                            &CanvasKind::Draw,
                            dim_linear_angle(bdl, cinfo, Status::default()),
                        );
                        Some(())
                    });
                }
            }
            Custom => {
                if let Some(points) = avb.on_creation.clone() {
                    let vlen = points.0.len() as i64;
                    for idx in 0..vlen {
                        let vertex = points.0.get(idx).get_vertex().curr;
                        let vertex_next = if idx + 1 == vlen {
                            points.1
                        } else {
                            points.0.get(idx + 1).get_vertex().curr
                        };
                        avb.canvases.draw_path(
                            &CanvasKind::Draw,
                            (
                                point_path(vertex, scale),
                                Pattern::Point,
                                create_colors,
                                vec![],
                            ),
                        );
                        avb.canvases.draw_path(
                            &CanvasKind::Draw,
                            (
                                line_path(vertex, vertex_next),
                                Pattern::OnCreation,
                                create_colors,
                                vec![],
                            ),
                        );
                        SegBundle::new(vertex, vertex_next).and_then(|bdl| {
                            avb.canvases.draw_path(
                                &CanvasKind::Draw,
                                dim_linear_angle(bdl, cinfo, Status::default()),
                            );
                            Some(())
                        });
                    }
                }
            }
        },

        _ => (),
    }

    // let end_time = performance.now();
    // log!("Rendering time: {:.2} ms", end_time - start_time);
}
