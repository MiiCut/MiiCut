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
pub mod cnc_link;
pub mod dimensions;
pub mod dom;
pub mod inputs;
pub mod math;
pub mod prefab;
pub mod shape;
pub mod shapes;
pub mod types;
pub mod undoredo;
use crate::cnc_link::CncLink;
use crate::dom::*;
use crate::math::*;
use crate::prefab::get_fill_color;
use crate::prefab::get_stroke_color;
use crate::prefab::get_text_colors;
use crate::prefab::line_path;
use crate::types::EUId;
use crate::types::VUId;
use canvas::{
    CanvasKind, CanvasText, CanvasTextConfig, Canvases, Color, Pattern, TextAlign, TextPos,
};
use clipboard::*;
use dimensions::dim_hv;
use dimensions::dim_radius;
use inputs::Keys;
use inputs::SystemMouse;
use inputs::*;
use kurbo::{Size, Vec2};
use prefab::get_vertices_colors;
use prefab::point_path;
use shape::ClosedShape;
use shapes::DataSet;
use std::cell::RefMut;
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
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    window, Document, Element, Event, FileReader, HtmlCanvasElement, HtmlElement, HtmlInputElement,
    KeyboardEvent, MouseEvent, WheelEvent, Window,
};

type RefAV = Rc<RefCell<AppVars>>;

fn cnc_send(av: RefAV, cmd: String) {
    // Clone the Rc while the RefCell borrow is alive, then drop the borrow before async.
    let cnc = { av.borrow().cnc.clone() };

    if let Some(cnc) = cnc {
        spawn_local(async move {
            let _ = cnc.send_http_cmd(&cmd).await;
        });
    } else {
        log!("[CNC] Not connected");
    }
}

fn main() {
    console_error_panic_hook::set_once();
    let window = window().expect("no global `window` exists");
    create_app_vars(window).expect("Could not access the document");
}

#[allow(dead_code)]
struct AppVars {
    element_on_creation: Option<(Icons, Vec<Vec2>)>,
    dataset: DataSet,
    clipboard: Clipboard,
    undo_redo: UndoRedo,

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

    //
    cnc: Option<Rc<CncLink>>,
}

impl AppVars {
    fn esc_pressed(&mut self) {
        self.element_on_creation = None;
        self.go_to_arrow_tool();
    }
    fn ctrl_c_pressed(&mut self) {
        if let Icons::Arrow = self.icon_selected.clone() {
            if self.dataset.shapes_selected.len() == 1 {
                let eid = *self.dataset.shapes_selected.iter().next().unwrap();
                if let Some(elem) = self.dataset.get_element(eid) {
                    self.clipboard
                        .copy(elem.clone(), self.user_ui.pointer.clone());
                }
            }
        }
    }
    fn ctrl_v_pressed(&mut self) {
        if let Icons::Arrow = self.icon_selected.clone() {
            if self.dataset.shapes_selected.len() == 1 {
                self.clipboard.paste(self.user_ui.pointer.clone());
            }
        }
    }
    fn del_back_pressed(&mut self) {
        if let Icons::Arrow = self.icon_selected {
            if !self.dataset.delete_selected_elements() {
                let vs_sel: Vec<(EUId, VUId)> =
                    self.dataset.vertices_selected.iter().copied().collect();
                if vs_sel.len() == 1 {
                    self.dataset.delete_vertex(vs_sel[0].0, vs_sel[0].1);
                    self.dataset.calc_final_polygon();
                }
            }
        }
    }
    fn space_pressed(&mut self) {
        if let Icons::Arrow = self.icon_selected {
            // Change operation of the selected shape
            let elems_sel: Vec<EUId> = self.dataset.shapes_selected.iter().copied().collect();
            if elems_sel.len() == 1 {
                if let Some(elem) = self.dataset.get_element_mut(elems_sel[0]) {
                    elem.op_next();
                    self.dataset.calc_final_polygon();
                    return;
                }
            }

            // If #vertices selected = 2:
            // same shape, try to add vertex between
            // different shape: bind/unbind
            let vs_sel: Vec<(EUId, VUId)> =
                self.dataset.vertices_selected.iter().copied().collect();
            if vs_sel.len() == 2 {
                let (eid1, vid1) = vs_sel[0];
                let (eid2, vid2) = vs_sel[1];
                if eid1 == eid2 {
                    // Vertices belong to the same shape
                    self.dataset.create_vertices_between(eid1, vid1, eid2, vid2);
                } else {
                    self.dataset.bind_unbind_vertices(eid1, vid1, eid2, vid2);
                }
                return;
            }

            // If #vertices selected = 1: change apex type
            if vs_sel.len() == 1 {
                let (eid, vid) = vs_sel[0];
                if let Some(elem) = self.dataset.get_element_mut(eid) {
                    if let Some(v) = elem.get_vertex_mut(&vid) {
                        // Change the apex type of the vertex
                        v.change_apex_type();
                        elem.set_bezpath();
                        self.dataset.calc_final_polygon();
                    }
                }
            }
        }
    }
    fn s_pressed(&mut self) {
        self.user_ui.snap.next_linear();
    }
    fn a_pressed(&mut self) {
        self.user_ui.snap.next_angle();
    }

    fn change_vertex_radius(&mut self, inc: i32) {
        if let Icons::Arrow = self.icon_selected {
            let vs_sel: Vec<(EUId, VUId)> =
                self.dataset.vertices_selected.iter().copied().collect();
            if vs_sel.len() == 1 {
                let (eid, vid) = vs_sel[0];
                if let Some(elem) = self.dataset.get_element_mut(eid) {
                    if let Some(v) = elem.get_vertex_mut(&vid) {
                        if let Some(round) = v.rounded.as_mut() {
                            log!("round: {}", round);
                            let res = *round as i32 + inc;
                            if res > 0 {
                                *round = res as u32;
                                elem.set_bezpath();
                                self.dataset.calc_final_polygon();
                            }
                        }
                    }
                }
            }
        }
    }
    fn arrow_up_pressed(&mut self) {
        let inc = self.user_ui.snap.linear() as i32;
        self.change_vertex_radius(inc);
    }
    fn arrow_down_pressed(&mut self) {
        let inc = -(self.user_ui.snap.linear() as i32);
        self.change_vertex_radius(inc);
    }

    // fn arrow_left_pressed(&mut self) {
    //     self.user_ui.snap.angle();
    // }

    // fn arrow_right_pressed(&mut self) {
    //     self.user_ui.snap.angle();
    // }

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
        self.user_ui.update_ui(
            get_element_width(&self.left_panel),
            get_element_height(&self.top_menu),
            self.canvases.get_drawing_offset(),
            self.canvases.get_drawing_scale(),
            &mouse_event,
            sys_mouse,
        )
    }
    fn set_element_select_vertex(&mut self) -> bool {
        self.dataset.select_vertices(&self.user_ui)
    }
    fn set_select_elements(&mut self) {
        self.dataset.select_elements(&self.user_ui);
    }
    fn set_element_highlight_vertex(&mut self) -> bool {
        self.dataset.highlight_vertices(&self.user_ui)
    }
    fn set_highlight_elements(&mut self) {
        self.dataset.highlight_elements(&self.user_ui);
    }
    fn set_move_elements(&mut self) -> bool {
        self.dataset.move_elements(&self.user_ui)
    }
    fn set_move_vertices_selected(&mut self) -> bool {
        if let Some((last_eid, last_vid)) = self.dataset.last_vertex_selected {
            self.dataset.get_element_mut(last_eid).map(|e| {
                return e.move_vertex(last_vid, &self.user_ui);
            });
        }
        false
    }
    fn move_canvas(&mut self) {
        self.canvases
            .move_drawing_offset(self.user_ui.pointer.curr - self.user_ui.pointer.saved);
    }
}

///////////////
// Initialization
fn create_app_vars(window: Window) -> Result<(), JsValue> {
    log!("Creating application variables");
    log!("Initializing icons");
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
    user_icons.insert(Square);
    user_icons.insert(Oblong);
    user_icons.insert(Poly);

    let set = DataSet::new();
    let cnc: Option<Rc<CncLink>> = CncLink::connect("http://192.168.1.36", "ws://192.168.1.36:81/")
        .ok()
        .map(Rc::new);

    let app_vars = Rc::new(RefCell::new(AppVars {
        element_on_creation: None,
        dataset: set,
        clipboard: Clipboard::new(),
        undo_redo: UndoRedo::new(),
        window: window,
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
        cnc,
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
    render_drawing(av.clone());

    Ok(())
}

fn init_window(av: RefAV) -> Result<(), JsValue> {
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
fn init_icons(av: RefAV) -> Result<(), JsValue> {
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
fn init_canvas(av: RefAV) -> Result<(), JsValue> {
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
fn init_menu(av: RefAV) -> Result<(), JsValue> {
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

fn init_status(av: RefAV) -> Result<(), JsValue> {
    let pam = av.borrow_mut();
    let _document = pam.document.clone();

    Ok(())
}
fn set_callback(
    av: RefAV,
    event_str: String,
    element: &Element,
    callbacka: Box<dyn Fn(RefAV, Event) + 'static>,
) -> Result<(), JsValue> {
    let event_str_cloned = event_str.clone();

    let cb = Box::new(move |av: RefAV, e: Event| {
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
fn convert_svg_to_shapes(_av: RefAV, svg_data: String) {
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

fn update(av: RefAV, user_action: UserAction) -> Result<(), MyError> {
    let mut avb = av.borrow_mut();
    match avb.icon_selected {
        Icons::Arrow => match user_action {
            UserAction::ClickDown(button, _clicks) => {
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
                    avb.dataset.calc_final_polygon();
                } else {
                    // Elements and vertices selection are mutual exclusive
                    // Moving is done on selected objects
                    // Move Elements
                    match btn {
                        MouseButton::Left => {
                            if !avb.set_move_elements() {
                                // If no elements moved, move vertices
                                avb.set_move_vertices_selected();
                            }
                        }
                        MouseButton::Right => {
                            avb.move_canvas();
                        }
                        _ => {}
                    }
                }
            }
            UserAction::ClickUp(_, _) => {
                avb.dataset.calc_final_polygon();
            }
        },
        Icons::Disc | Icons::Square | Icons::Oblong | Icons::Poly => {
            let pointer_pos = avb.user_ui.pointer.curr;
            match user_action {
                UserAction::ClickDown(button, clicks) => {
                    if button == MouseButton::Left {
                        if let Some((cs, mut vs)) = avb.element_on_creation.clone() {
                            match cs {
                                Icons::Disc | Icons::Square | Icons::Oblong => {
                                    if vs.len() == 0 {
                                        vs.push(pointer_pos);
                                        avb.element_on_creation = Some((cs, vs));
                                    } else {
                                        vs.push(pointer_pos);
                                        let o_e = ClosedShape::new(cs, &vs);
                                        if let Some(e) = o_e {
                                            avb.dataset.push_element(e);
                                            avb.element_on_creation = Some((cs, vec![]));
                                        } else {
                                            log!("Error creating element: {:?}", cs);
                                        }
                                    }
                                }
                                Icons::Poly => {
                                    if let Some(_) = vs.first() {
                                        if clicks == 2 {
                                            let o_e = ClosedShape::new(cs, &vs);
                                            if let Some(e) = o_e {
                                                avb.dataset.push_element(e);
                                                avb.element_on_creation = Some((cs, vec![]));
                                            } else {
                                                log!("Error creating element: {:?}", cs);
                                            }
                                        } else {
                                            vs.push(pointer_pos);
                                            avb.element_on_creation = Some((cs, vs));
                                        }
                                    } else {
                                        vs.push(pointer_pos);
                                        avb.element_on_creation = Some((cs, vs));
                                    }
                                }
                                _ => {}
                            }
                        }
                    } else {
                        if button == MouseButton::Right {
                            if let Some((cs, mut vs)) = avb.element_on_creation.clone() {
                                match cs {
                                    Icons::Poly => {
                                        if vs.len() == 0 {
                                            // Right click to cancel the creation
                                            avb.element_on_creation = None;
                                            avb.go_to_arrow_tool();
                                        } else {
                                            vs.pop();
                                            avb.element_on_creation = Some((cs, vs));
                                        }
                                    }
                                    _ => {
                                        // Right click to cancel the creation
                                        avb.element_on_creation = None;
                                        avb.go_to_arrow_tool();
                                    }
                                }
                            }
                        }
                    }
                }
                UserAction::ClickUp(_, _) => {
                    avb.dataset.calc_final_polygon();
                }
                _ => (),
            }
        }
    }
    Ok(())
}

fn on_mouse_move(av: RefAV, event: Event) {
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
fn on_mouse_down(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.clone().dyn_into::<MouseEvent>() {
        let clicks = mouse_event.detail();
        let ua = av
            .borrow_mut()
            .update_inputs(mouse_event, SystemMouse::Down(clicks));
        // Hide the context menu when clicking elsewhere
        display_html_element(DOMElements::ContextMenuShape, false);
        if let Some(e) = update(av.clone(), ua).err() {
            log!("ERROR: {}", e);
        }
    }
    // update_status_bar(&mut pam);
    render_drawing(av);
}
fn on_mouse_up(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.clone().dyn_into::<MouseEvent>() {
        let clicks = mouse_event.detail();
        let ua = av
            .borrow_mut()
            .update_inputs(mouse_event, SystemMouse::Up(clicks));
        if let Some(e) = update(av.clone(), ua).err() {
            log!("ERROR: {}", e);
        }
    }
    render_drawing(av);
}

fn on_mouse_wheel(av: RefAV, event: Event) {
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
fn on_mouse_enter(av: RefAV, _event: Event) {
    let mut avb = av.borrow_mut();
    avb.pointer_on_canvas = true;
    let curr_pos = avb.user_ui.pointer.curr;
    avb.user_ui.pointer.saved = curr_pos;
    // if let Some(elem) = &mut avb.element_on_creation {
    //     elem.move_drawable(curr_pos);
    //     elem.save_vertices_positions();
    // }
    drop(avb);
    render_drawing(av);
}
fn on_mouse_leave(av: RefAV, _event: Event) {
    let mut avb = av.borrow_mut();
    avb.pointer_on_canvas = false;
    avb.element_on_creation = None;
    avb.go_to_arrow_tool();
    drop(avb);
    render_drawing(av);
}

fn on_context_menu(_av: RefAV, event: Event) {
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
fn resize_canvases(av: RefAV) {
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
fn on_window_resize(av: RefAV, _event: Event) {
    resize_canvases(av.clone());
    render_drawing(av);
}
fn on_window_click(_pa: RefAV, _event: Event) {}
fn on_window_keydown(av: RefAV, event: Event) {
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
                Space => {
                    avb.space_pressed();
                }
                ArrowUp => {
                    avb.arrow_up_pressed();
                }
                ArrowDown => {
                    avb.arrow_down_pressed();
                }
                ArrowLeft => {
                    // Jog X -1mm (responses will arrive on websocket)
                    let cmd = "$J=G91 X-1 F200".to_string();
                    drop(avb);
                    cnc_send(av.clone(), cmd);
                    render_drawing(av);
                    return;
                }
                ArrowRight => {
                    // Jog X +1mm
                    let cmd = "$J=G91 X1 F200".to_string();
                    drop(avb);
                    cnc_send(av.clone(), cmd);
                    render_drawing(av);
                    return;
                }
                _ => (),
            };
            drop(avb);
            render_drawing(av);
        }
    }
}
fn on_window_keyup(av: RefAV, event: Event) {
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
            drop(avb);
            render_drawing(av);
        }
    }
}
///////////////
// Icons events
fn on_icon_click(av: RefAV, event: Event) {
    let mut avb = av.borrow_mut();
    if let Some(target) = event.target() {
        if let Some(element) = wasm_bindgen::JsCast::dyn_ref::<Element>(&target) {
            if let Some(id) = element.get_attribute("id") {
                if let Some(icon) = avb.user_icons.iter().find(|&&k| k.id() == id).cloned() {
                    avb.icon_selected = icon;
                    avb.dataset.shapes_highlighted.clear();
                    avb.dataset.shapes_selected.clear();
                    avb.user_icons
                        .iter()
                        .for_each(|icon| avb.html_deselect_icons(*icon));
                    avb.html_select_icon(icon);
                    match avb.icon_selected {
                        Icons::Arrow => {
                            avb.element_on_creation = None;
                        }
                        _ => avb.element_on_creation = Some((avb.icon_selected, vec![])),
                    }
                }
            }
        }
    }
    drop(avb);
    render_drawing(av);
}
fn on_icon_mouseover(av: RefAV, event: Event) {
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
fn on_icon_mouseout(av: RefAV, _event: Event) {
    av.borrow_mut()
        .tooltip
        .style()
        .set_property("display", "none")
        .expect("Failed to set display property");
}

///////////////
// Rendering
fn reset_origin(av: RefAV) {
    av.borrow_mut().canvases.reset_origin();
}

fn draw_grid_and_rules(av: RefAV) {
    av.borrow_mut().canvases.draw_origin();
}

fn may_be_draw_radiuses(avb: &RefMut<'_, AppVars>, e: &ClosedShape) {
    let text_color = get_text_colors().stroke_color;
    let text_cfg = CanvasTextConfig::new(text_color, 0., TextAlign::Center, 14, 0.8);
    for apex_type in e.get_vertices().get_apices().iter() {
        if let ApexType::Arc { s, c, e: _e } = apex_type {
            let r = (*s - *c).length();
            let text2 = CanvasText::new(
                format!("R{:.0}", r),
                TextPos::PosCustom(*c),
                text_cfg.clone(),
            );
            avb.canvases.draw_text(&CanvasKind::Draw, &text2);
        }
    }
}

fn draw_dimensions(avb: &RefMut<'_, AppVars>, e: &ClosedShape) {
    match e.get_shape_type() {
        Icons::Disc => {
            let v: Vec<Vec2> = e.get_vertices().iter().map(|(_, v)| v.curr).collect();
            if v.len() == 2 {
                if let Some(seg) = SegBundle::new(v[0], v[1]) {
                    // Draw the radius segment
                    let (path, pattern, colors, text) =
                        dim_radius(seg, avb.canvases.get_canvas_infos(), true, true);
                    avb.canvases.draw_path(
                        &CanvasKind::Draw,
                        &path,
                        pattern,
                        colors.fill_color,
                        colors.stroke_color,
                        text,
                    );
                }
            }
        }
        Icons::Square => {
            for (v1, v2) in e
                .get_vertices()
                .iter()
                .zip(e.get_vertices().iter().cycle().skip(1))
                .take(2)
                .map(|(v1, v2)| (v1.1.curr, v2.1.curr))
            {
                if let Some(seg) = SegBundle::new(v1, v2) {
                    let (path, pattern, colors, text) =
                        dim_hv(seg, avb.canvases.get_canvas_infos());
                    avb.canvases.draw_path(
                        &CanvasKind::Draw,
                        &path,
                        pattern,
                        colors.fill_color,
                        colors.stroke_color,
                        text,
                    );
                }
            }
        }
        Icons::Oblong => {
            let v: Vec<Vec2> = e.get_vertices().iter().map(|(_, v)| v.curr).collect();
            if v.len() == 3 {
                if let Some(seg1) = SegBundle::new(v[0], v[1]) {
                    // Draw the main segment
                    let (path, pattern, colors, text) =
                        dim_radius(seg1, avb.canvases.get_canvas_infos(), false, true);
                    avb.canvases.draw_path(
                        &CanvasKind::Draw,
                        &path,
                        pattern,
                        colors.fill_color,
                        colors.stroke_color,
                        text,
                    );
                    // Draw the radius segment
                    if let Some(seg2) = SegBundle::new(seg1.m, v[2]) {
                        let (path, pattern, colors, text) =
                            dim_radius(seg2, avb.canvases.get_canvas_infos(), true, false);
                        avb.canvases.draw_path(
                            &CanvasKind::Draw,
                            &path,
                            pattern,
                            colors.fill_color,
                            colors.stroke_color,
                            text,
                        );
                    }
                }
            }
        }
        Icons::Poly => {
            for (v1, v2) in e
                .get_vertices()
                .iter()
                .zip(e.get_vertices().iter().cycle().skip(1))
                .map(|(v1, v2)| (v1.1.curr, v2.1.curr))
            {
                if let Some(seg) = SegBundle::new(v1, v2) {
                    let (path, pattern, colors, text) =
                        dim_hv(seg, avb.canvases.get_canvas_infos());
                    avb.canvases.draw_path(
                        &CanvasKind::Draw,
                        &path,
                        pattern,
                        colors.fill_color,
                        colors.stroke_color,
                        text,
                    );
                }
            }
        }
        Icons::Arrow => (),
    }
}

fn draw_vertices(avb: &RefMut<'_, AppVars>, e: &ClosedShape) {
    for (vid, vertex) in e.get_vertices().iter() {
        let vid_sel = avb
            .dataset
            .vertices_selected
            .iter()
            .any(|&(_, sel_vid)| &sel_vid == vid);
        let vid_high = avb
            .dataset
            .vertices_highlighted
            .iter()
            .any(|&(_, high_vid)| &high_vid == vid);

        let colors = get_vertices_colors(vid_sel, vid_high);
        avb.canvases.draw_path(
            &CanvasKind::Draw,
            &point_path(vertex.curr, 1.),
            Pattern::Point,
            colors.fill_color,
            colors.stroke_color,
            vec![],
        );
    }
}

fn draw_paths_creation(avb: &RefMut<'_, AppVars>, e: &ClosedShape) {
    let stroke_color = get_stroke_color(false, false);
    let path = e.get_bezpath();
    avb.canvases.draw_path(
        &CanvasKind::Draw,
        path,
        Pattern::OnCreation,
        Color::Gray95,
        stroke_color,
        vec![],
    );
}

fn draw_paths_set(avb: &RefMut<'_, AppVars>, eid: &EUId, e: &ClosedShape) {
    let stroke_color = get_stroke_color(
        avb.dataset.shapes_selected.contains(eid),
        avb.dataset.shapes_highlighted.contains(eid),
    );
    let fill_color = get_fill_color(
        avb.dataset.shapes_selected.contains(eid),
        avb.dataset.shapes_highlighted.contains(eid),
    );
    let path = e.get_bezpath();
    avb.canvases.draw_path(
        &CanvasKind::Draw,
        path,
        Pattern::Point,
        fill_color,
        stroke_color,
        vec![],
    );
}

fn render_drawing(av: RefAV) {
    let mut avb = av.borrow_mut();
    // use IconsShapes::*;
    // Get the Performance API
    // let performance = window().unwrap().performance().unwrap();
    // let start_time = performance.now();

    avb.canvases.clear_main_canvas();

    // Draw full contour
    let full_path: &Vec<kurbo::BezPath> = avb.dataset.get_final_paths();
    avb.canvases.draw_closed_path(
        &CanvasKind::Draw,
        full_path,
        Pattern::Composed(true),
        Color::Black,
        Color::Gray20,
        vec![],
    );

    if !avb.user_ui.keys_states.alt_pressed {
        // Draw the elements (shapes outlines)
        for (eid, e) in avb.dataset.shapes.iter() {
            draw_paths_set(&avb, eid, e);
        }

        // Draw radiuses -if any- for square and polygon
        for e in avb.dataset.shapes.values() {
            match e.get_shape_type() {
                Icons::Square | Icons::Poly => may_be_draw_radiuses(&avb, e),
                _ => (),
            }
        }
        // Draw vertices, dimensions and get the bind on the same time
        let mut binds = Binding::<(EUId, VUId)>::new();
        for (eid, e) in avb.dataset.shapes.iter() {
            draw_vertices(&avb, e);
            draw_dimensions(&avb, e);
            for (vid, vertex) in e.get_vertices().iter() {
                // If the vertex is bound to other vertices, store the binding
                binds.extend(vertex.bind.iter().map(|(eid2, vid2)| {
                    Couple((eid.clone(), vid.clone()), (eid2.clone(), vid2.clone()))
                }));
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
                        let (path, pattern, colors, text) =
                            dim_hv(seg, avb.canvases.get_canvas_infos());
                        avb.canvases.draw_path(
                            &CanvasKind::Draw,
                            &path,
                            pattern,
                            colors.fill_color,
                            colors.stroke_color,
                            text,
                        );
                    }
                }
            }
        }
    }

    // CLIPBOARD
    // if let Some(item) = avb.clipboard.get_paste() {
    //     match item {}
    // }

    // Draw element on creation if any
    if let Some((cs, mut vs)) = avb.element_on_creation.clone() {
        match cs {
            Icons::Disc | Icons::Square | Icons::Oblong => {
                if vs.len() == 1 {
                    vs.push(avb.user_ui.pointer.curr);
                    if let Some(e) = ClosedShape::new(cs, &vs) {
                        draw_paths_creation(&avb, &e);
                        draw_vertices(&avb, &e);
                        draw_dimensions(&avb, &e);
                    }
                }
            }
            Icons::Poly => {
                if vs.len() > 2 {
                    vs.push(avb.user_ui.pointer.curr);
                    if let Some(e) = ClosedShape::new(cs, &vs) {
                        draw_paths_creation(&avb, &e);
                        draw_vertices(&avb, &e);
                        draw_dimensions(&avb, &e);
                    }
                } else {
                    vs.push(avb.user_ui.pointer.curr);
                    if vs.len() >= 2 {
                        let colors = get_vertices_colors(false, false);
                        for (i, v) in vs.iter().enumerate() {
                            if i < vs.len() - 1 {
                                // Line
                                avb.canvases.draw_path(
                                    &CanvasKind::Draw,
                                    &line_path(vs[i], vs[i + 1]),
                                    Pattern::OnCreation,
                                    colors.fill_color,
                                    colors.stroke_color,
                                    vec![],
                                );
                                // Dimension
                                if let Some(seg) = SegBundle::new(vs[i], vs[i + 1]) {
                                    let (path, pattern, colors, text) =
                                        dim_hv(seg, avb.canvases.get_canvas_infos());
                                    avb.canvases.draw_path(
                                        &CanvasKind::Draw,
                                        &path,
                                        pattern,
                                        colors.fill_color,
                                        colors.stroke_color,
                                        text,
                                    );
                                }
                            }
                            avb.canvases.draw_path(
                                &CanvasKind::Draw,
                                &point_path(*v, 1.),
                                Pattern::Point,
                                colors.fill_color,
                                colors.stroke_color,
                                vec![],
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Draw the pointer
    avb.canvases.draw_pointer(avb.user_ui.pointer.curr);

    drop(avb);
    render_informations(av);
}

fn render_informations(av: RefAV) {
    let avb = av.borrow_mut();
    // avb.canvases.clear_background_canvas();
    let c_size = avb.canvases.get_canvas_size();
    let pos = avb.user_ui.pointer.curr;
    avb.canvases.direct_text(
        &CanvasKind::Draw,
        &CanvasText::new(
            format!("({:.2},{:.2})", pos.x, pos.y),
            TextPos::PosCustom(Vec2::new(c_size.width - 200., c_size.height - 10.)),
            CanvasTextConfig::new(Color::Rules, 0., TextAlign::Left, 20, 0.4),
        ),
    );
    avb.canvases.direct_text(
        &CanvasKind::Draw,
        &CanvasText::new(
            format!("Snap linear: {:.0} mm", avb.user_ui.snap.linear(),),
            TextPos::PosCustom(Vec2::new(0., c_size.height - 30.)),
            CanvasTextConfig::new(Color::Rules, 0., TextAlign::Left, 20, 0.4),
        ),
    );
    avb.canvases.direct_text(
        &CanvasKind::Draw,
        &CanvasText::new(
            format!("Snap angle: {:.0}°", avb.user_ui.snap.angle()),
            TextPos::PosCustom(Vec2::new(0., c_size.height - 10.)),
            CanvasTextConfig::new(Color::Rules, 0., TextAlign::Left, 20, 0.4),
        ),
    );
}
