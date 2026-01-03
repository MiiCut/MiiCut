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
pub mod gcode;
pub mod inputs;
pub mod math;
pub mod prefab;
pub mod shape;
pub mod shapes;
pub mod types;
pub mod undoredo;

use crate::canvas::Canvas;
use crate::cnc_link::CncLink;
use crate::dom::*;
use crate::gcode::gcode_to_segments;
use crate::gcode::Seg;
use crate::math::*;
use crate::prefab::line_path;
use crate::shapes::multipolygon_to_plasma_gcode;
use crate::shapes::CutParams;
use crate::types::EUId;
use crate::types::VUId;
use canvas::{CanvasKind, CanvasText, CanvasTextConfig, Color, Pattern, TextAlign, TextPos};
use dimensions::dim_hv;
use inputs::Keys;
use inputs::SystemMouse;
use inputs::*;
use kurbo::{BezPath, PathEl, Point};
use kurbo::{Size, Vec2};
use prefab::get_vertices_colors;
use prefab::point_path;
use shape::ClosedShape;
use std::collections::HashSet;
use std::str::FromStr;
use std::{cell::RefCell, rc::Rc};
use svg::node::element::path::Data;
use svg::read;
use types::Binding;
use types::Couple;
use types::SegBundle;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    window, Document, Element, Event, FileReader, HtmlCanvasElement, HtmlElement, HtmlInputElement,
    KeyboardEvent, MouseEvent, WheelEvent, Window,
};
type RefAV = Rc<RefCell<AppVars>>;

fn main() {
    console_error_panic_hook::set_once();
    let window = window().expect("no global `window` exists");
    create_app_vars(window).expect("Could not access the document");
}

#[allow(dead_code)]
struct AppVars {
    element_on_creation: Option<(Icons, Vec<Vec2>)>,

    // DOM
    window: Window,
    document: Document,
    top_menu: HtmlElement,
    left_panel: HtmlElement,
    user_icons: HashSet<Icons>,
    tooltip: HtmlElement,
    icon_selected: Icons,
    canvases: [Canvas; CanvasKind::COUNT],
    active_canvas: CanvasKind,
    active_view: Tabs,

    last_gcode: Option<String>,
    gcode_auto_center: bool,
    gcode_auto_fit: bool,
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
            let canvas_user = self.get_user_canvas_mut();
            if canvas_user.dataset.shapes_selected.len() == 1 {
                let eid = *canvas_user.dataset.shapes_selected.iter().next().unwrap();
                if let Some(elem) = canvas_user.dataset.get_element(eid) {
                    canvas_user
                        .clipboard
                        .copy(elem.clone(), canvas_user.get_user_ui().pointer.clone());
                }
            }
        }
    }
    fn ctrl_v_pressed(&mut self) {
        if let Icons::Arrow = self.icon_selected.clone() {
            let canvas_user = self.get_user_canvas_mut();
            if canvas_user.dataset.shapes_selected.len() == 1 {
                canvas_user
                    .clipboard
                    .paste(canvas_user.get_user_ui().pointer.clone());
            }
        }
    }
    fn del_back_pressed(&mut self) {
        if let Icons::Arrow = self.icon_selected {
            let canvas_user = self.get_user_canvas_mut();
            if !canvas_user.dataset.delete_selected_elements() {
                let vs_sel: Vec<(EUId, VUId)> = canvas_user
                    .dataset
                    .vertices_selected
                    .iter()
                    .copied()
                    .collect();
                if vs_sel.len() == 1 {
                    canvas_user.dataset.delete_vertex(vs_sel[0].0, vs_sel[0].1);
                    canvas_user.dataset.calc_final_polygon();
                }
            }
        }
    }
    fn space_pressed(&mut self) {
        if let Icons::Arrow = self.icon_selected {
            let canvas_user = self.get_user_canvas_mut();
            // Change operation of the selected shape
            let elems_sel: Vec<EUId> = canvas_user
                .dataset
                .shapes_selected
                .iter()
                .copied()
                .collect();
            if elems_sel.len() == 1 {
                if let Some(elem) = canvas_user.dataset.get_element_mut(elems_sel[0]) {
                    elem.op_next();
                    canvas_user.dataset.calc_final_polygon();
                    return;
                }
            }

            // If #vertices selected = 2:
            // same shape, try to add vertex between
            // different shape: bind/unbind
            let vs_sel: Vec<(EUId, VUId)> = canvas_user
                .dataset
                .vertices_selected
                .iter()
                .copied()
                .collect();
            if vs_sel.len() == 2 {
                let (eid1, vid1) = vs_sel[0];
                let (eid2, vid2) = vs_sel[1];
                if eid1 == eid2 {
                    // Vertices belong to the same shape
                    canvas_user
                        .dataset
                        .create_vertices_between(eid1, vid1, eid2, vid2);
                } else {
                    canvas_user
                        .dataset
                        .bind_unbind_vertices(eid1, vid1, eid2, vid2);
                }
                return;
            }

            // If #vertices selected = 1: change apex type
            if vs_sel.len() == 1 {
                let (eid, vid) = vs_sel[0];
                if let Some(elem) = canvas_user.dataset.get_element_mut(eid) {
                    if let Some(v) = elem.get_vertex_mut(&vid) {
                        // Change the apex type of the vertex
                        v.change_apex_type();
                        elem.set_bezpath();
                        canvas_user.dataset.calc_final_polygon();
                    }
                }
            }
        }
    }
    fn s_pressed(&mut self) {
        self.canvases[self.active_canvas.idx()]
            .get_user_ui_mut()
            .snap
            .next_linear();
    }
    fn a_pressed(&mut self) {
        self.canvases[self.active_canvas.idx()]
            .get_user_ui_mut()
            .snap
            .next_angle();
    }

    fn change_vertex_radius(&mut self, inc: i32) {
        if let Icons::Arrow = self.icon_selected {
            let canvas_user = self.get_user_canvas_mut();
            let vs_sel: Vec<(EUId, VUId)> = canvas_user
                .dataset
                .vertices_selected
                .iter()
                .copied()
                .collect();
            if vs_sel.len() == 1 {
                let (eid, vid) = vs_sel[0];
                if let Some(elem) = canvas_user.dataset.get_element_mut(eid) {
                    if let Some(v) = elem.get_vertex_mut(&vid) {
                        if let Some(round) = v.rounded.as_mut() {
                            log!("round: {}", round);
                            let res = *round as i32 + inc;
                            if res > 0 {
                                *round = res as u32;
                                elem.set_bezpath();
                                canvas_user.dataset.calc_final_polygon();
                            }
                        }
                    }
                }
            }
        }
    }
    fn arrow_up_pressed(&mut self) {
        let inc = self.canvases[self.active_canvas.idx()]
            .get_user_ui()
            .snap
            .linear() as i32;
        self.change_vertex_radius(inc);
    }
    fn arrow_down_pressed(&mut self) {
        let inc = -(self.canvases[self.active_canvas.idx()]
            .get_user_ui()
            .snap
            .linear() as i32);
        self.change_vertex_radius(inc);
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

    fn set_element_select_vertex(&mut self) -> bool {
        let canvas_user = self.get_user_canvas_mut();
        canvas_user
            .dataset
            .select_vertices(canvas_user.get_user_ui().draw_pos)
    }
    fn set_select_elements(&mut self) {
        self.get_user_canvas_mut().select_elements();
    }
    fn set_highlight_elements(&mut self) {
        self.get_user_canvas_mut().highlight_elements();
    }
    fn set_element_highlight_vertex(&mut self) -> bool {
        self.get_user_canvas_mut().highlight_vertices()
    }
    fn set_move_elements(&mut self) -> bool {
        self.get_user_canvas_mut().move_elements()
    }
    fn set_move_vertices_selected(&mut self) -> bool {
        self.get_user_canvas_mut().move_vertices_selected()
    }

    fn get_user_canvas(&self) -> &Canvas {
        match self.active_canvas {
            CanvasKind::Gcode => &self.canvases[CanvasKind::Gcode.idx()],
            CanvasKind::Draw => &self.canvases[CanvasKind::Draw.idx()],
            CanvasKind::Background => &self.canvases[CanvasKind::Draw.idx()],
            CanvasKind::Grid => &self.canvases[CanvasKind::Draw.idx()],
        }
    }
    fn get_user_canvas_mut(&mut self) -> &mut Canvas {
        match self.active_canvas {
            CanvasKind::Gcode => &mut self.canvases[CanvasKind::Gcode.idx()],
            CanvasKind::Draw => &mut self.canvases[CanvasKind::Draw.idx()],
            CanvasKind::Background => &mut self.canvases[CanvasKind::Draw.idx()],
            CanvasKind::Grid => &mut self.canvases[CanvasKind::Draw.idx()],
        }
    }
    fn update_canvas_inputs(
        &mut self,
        mouse_event: MouseEvent,
        sys_mouse: SystemMouse,
    ) -> UserAction {
        let c_draw_origin = Vec2::new(
            get_element_width(&self.left_panel) as f64,
            get_element_height(&self.top_menu) as f64,
        );
        self.canvases[self.active_canvas.idx()].update_ui(c_draw_origin, &mouse_event, sys_mouse)
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
    let c_gcode: HtmlCanvasElement = init_element!(document, "gcodeCanvas", HtmlCanvasElement);
    let tooltip: HtmlElement = init_element!(document, "tooltip", HtmlElement);
    let left_panel: HtmlElement = init_element!(document, "left-panel", HtmlElement);
    let top_menu: HtmlElement = init_element!(document, "top-menu", HtmlElement);
    let canvases: [Canvas; CanvasKind::COUNT] = [
        Canvas::new(c_back, Size::new(3000., 1500.))?, // Background
        Canvas::new(c_grid, Size::new(3000., 1500.))?, // Grid
        Canvas::new(c_draw, Size::new(3000., 1500.))?, // Draw
        Canvas::new(c_gcode, Size::new(3000., 1500.))?, // Gcode
    ];
    let active_canvas = CanvasKind::Draw;
    let mut user_icons: HashSet<Icons> = HashSet::new();
    use Icons::*;
    user_icons.insert(Arrow);
    user_icons.insert(Disc);
    user_icons.insert(Square);
    user_icons.insert(Oblong);
    user_icons.insert(Poly);

    let cnc: Option<Rc<CncLink>> = CncLink::connect("http://192.168.1.36", "ws://192.168.1.36:81/")
        .ok()
        .map(Rc::new);

    let app_vars = Rc::new(RefCell::new(AppVars {
        element_on_creation: None,
        window: window,
        document,
        top_menu,
        left_panel,
        canvases,
        active_canvas,
        active_view: Tabs::Draw,
        //
        user_icons,
        tooltip,
        icon_selected: Icons::Arrow,

        last_gcode: None,
        gcode_auto_center: false,
        gcode_auto_fit: false,
        cnc,
    }));

    init_menu(app_vars.clone())?;
    // init_context_menu(app_vars.clone())?;
    init_tabs(app_vars.clone())?;
    init_gcode_splitter(app_vars.clone())?;
    init_icons(app_vars.clone())?;
    init_status(app_vars.clone())?;
    init_draw_canvas(app_vars.clone())?;
    init_gcode_canvas(app_vars.clone())?;
    init_window(app_vars.clone())?;
    resize_canvases(app_vars.clone());

    let av = app_vars;

    draw_reset_origin(av.clone());
    draw_grid_and_rules(av.clone());
    render_draw_view(av.clone());

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
fn init_tabs(av: RefAV) -> Result<(), JsValue> {
    // Comme pour tes icônes
    let tabs: HashSet<Tabs> = [Tabs::Draw, Tabs::Gcode].into_iter().collect();

    for tab in tabs.iter() {
        let el = tab
            .get_element()
            .unwrap_or_else(|| panic!("Tab element not found: {}", tab.id()));
        log!("Found tab element: {}", tab.id());
        // On passe la tab choisie via une closure (recommandé)
        let tab_copy = *tab;
        set_callback(
            av.clone(),
            "click".into(),
            &el,
            Box::new(move |av, _evt| on_tab_click(av, tab_copy)),
        )?;
    }

    Ok(())
}
fn init_gcode_splitter(av: RefAV) -> Result<(), wasm_bindgen::JsValue> {
    let window: Window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    let split: HtmlElement = document
        .get_element_by_id("gcode-split")
        .unwrap()
        .dyn_into()?;
    let left: HtmlElement = document
        .get_element_by_id("gcode-left")
        .unwrap()
        .dyn_into()?;
    let splitter: HtmlElement = document
        .get_element_by_id("gcode-splitter")
        .unwrap()
        .dyn_into()?;

    let dragging = std::rc::Rc::new(std::cell::Cell::new(false));

    // mousedown
    {
        let dragging = dragging.clone();
        let on_down = Closure::<dyn FnMut(MouseEvent)>::new(move |e: MouseEvent| {
            e.prevent_default();
            dragging.set(true);
        });
        splitter.add_event_listener_with_callback("mousedown", on_down.as_ref().unchecked_ref())?;
        on_down.forget();
    }

    // mousemove
    {
        let dragging = dragging.clone();
        let split = split.clone();
        let left = left.clone();
        let av2 = av.clone();

        let on_move = Closure::<dyn FnMut(MouseEvent)>::new(move |e: MouseEvent| {
            if !dragging.get() {
                return;
            }

            let rect = split.get_bounding_client_rect();
            let x = (e.client_x() as f64) - rect.left(); // position dans le split
            let w = rect.width();

            // clamps
            let min_left = 200.0;
            let min_right = 260.0;
            let max_left = (w - min_right).max(min_left);

            let new_left = x.clamp(min_left, max_left);

            let _ = left
                .style()
                .set_property("width", &format!("{:.0}px", new_left));

            // Important: recalcul tailles canvas (throttle simple: ici à chaque move)
            let mut avb = av2.borrow_mut();
            if let Some(left) = avb.document.get_element_by_id("gcode-left") {
                let rect = left.get_bounding_client_rect();
                let gw = rect.width().max(1.0) as u32;
                let gh = rect.height().max(1.0) as u32;
                avb.canvases[CanvasKind::Gcode.idx()].resize(gw, gh);
            }
        });

        window.add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref())?;
        on_move.forget();
    }

    // mouseup
    {
        let dragging = dragging.clone();
        let on_up = Closure::<dyn FnMut(MouseEvent)>::new(move |_| {
            dragging.set(false);
        });
        window.add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref())?;
        on_up.forget();
    }

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
fn init_draw_canvas(av: RefAV) -> Result<(), JsValue> {
    log!("Initializing canvas");
    let avb = av.borrow_mut();
    let c_draw = avb.canvases[CanvasKind::Draw.idx()].get_canvas();
    set_callback(
        av.clone(),
        "mousedown".into(),
        c_draw,
        Box::new(on_draw_mouse_down),
    )?;
    set_callback(
        av.clone(),
        "contextmenu".into(),
        c_draw,
        Box::new(on_draw_context_menu),
    )?;
    set_callback(
        av.clone(),
        "mousemove".into(),
        c_draw,
        Box::new(on_draw_mouse_move),
    )?;
    set_callback(
        av.clone(),
        "mouseup".into(),
        c_draw,
        Box::new(on_draw_mouse_up),
    )?;
    set_callback(
        av.clone(),
        "mouseenter".into(),
        c_draw,
        Box::new(on_draw_mouse_enter),
    )?;
    set_callback(
        av.clone(),
        "mouseleave".into(),
        c_draw,
        Box::new(on_draw_mouse_leave),
    )?;
    set_callback(
        av.clone(),
        "wheel".into(),
        c_draw,
        Box::new(on_draw_mouse_wheel),
    )?;
    Ok(())
}
fn init_gcode_canvas(av: RefAV) -> Result<(), JsValue> {
    let avb = av.borrow();
    let c = avb.canvases[CanvasKind::Gcode.idx()].get_canvas();

    set_callback(
        av.clone(),
        "wheel".into(),
        c,
        Box::new(on_gcode_mouse_wheel),
    )?;
    set_callback(
        av.clone(),
        "contextmenu".into(),
        c,
        Box::new(on_gcode_context_menu),
    )?;
    set_callback(
        av.clone(),
        "mousedown".into(),
        c,
        Box::new(on_gcode_mouse_down),
    )?;
    set_callback(
        av.clone(),
        "mousemove".into(),
        c,
        Box::new(on_gcode_mouse_move),
    )?;
    set_callback(av.clone(), "mouseup".into(), c, Box::new(on_gcode_mouse_up))?;
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
                    avb.canvases[CanvasKind::Draw.idx()].save_offset();
                    avb.canvases[CanvasKind::Draw.idx()]
                        .dataset
                        .save_elements_positions();
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
                    avb.canvases[CanvasKind::Draw.idx()]
                        .dataset
                        .calc_final_polygon();
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
                            avb.canvases[CanvasKind::Draw.idx()].move_offset();
                        }
                        _ => {}
                    }
                }
            }
            UserAction::ClickUp(_, _) => {
                avb.canvases[CanvasKind::Draw.idx()]
                    .dataset
                    .calc_final_polygon();
            }
        },
        Icons::Disc | Icons::Square | Icons::Oblong | Icons::Poly => {
            let pointer_pos = avb.canvases[CanvasKind::Draw.idx()]
                .get_user_ui()
                .pointer
                .curr;
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
                                            avb.canvases[CanvasKind::Draw.idx()]
                                                .dataset
                                                .push_element(e);
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
                                                avb.canvases[CanvasKind::Draw.idx()]
                                                    .dataset
                                                    .push_element(e);
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
                    avb.canvases[CanvasKind::Draw.idx()]
                        .dataset
                        .calc_final_polygon();
                }
                _ => (),
            }
        }
    }
    Ok(())
}

fn on_draw_mouse_move(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.clone().dyn_into::<MouseEvent>() {
        let ua = av
            .borrow_mut()
            .update_canvas_inputs(mouse_event, SystemMouse::Move);
        if let Some(e) = update(av.clone(), ua).err() {
            log!("ERROR: {}", e);
        }
    }
    render_draw_view(av);
}
fn on_draw_mouse_down(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.clone().dyn_into::<MouseEvent>() {
        let clicks = mouse_event.detail();
        let ua = av
            .borrow_mut()
            .update_canvas_inputs(mouse_event, SystemMouse::Down(clicks));
        // Hide the context menu when clicking elsewhere
        display_html_element(DOMElements::ContextMenuShape, false);
        if let Some(e) = update(av.clone(), ua).err() {
            log!("ERROR: {}", e);
        }
    }
    // update_status_bar(&mut pam);
    render_draw_view(av);
}
fn on_draw_mouse_up(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.clone().dyn_into::<MouseEvent>() {
        let clicks = mouse_event.detail();
        let ua = av
            .borrow_mut()
            .update_canvas_inputs(mouse_event, SystemMouse::Up(clicks));
        if let Some(e) = update(av.clone(), ua).err() {
            log!("ERROR: {}", e);
        }
    }
    render_draw_view(av);
}
fn on_draw_mouse_wheel(av: RefAV, event: Event) {
    if let Ok(wheel_event) = event.dyn_into::<WheelEvent>() {
        wheel_event.prevent_default();
        let mut avb = av.borrow_mut();

        let zoom_factor = 0.04;
        let old_draw_scale = avb.canvases[CanvasKind::Draw.idx()].get_scale();
        let old_draw_offset = avb.canvases[CanvasKind::Draw.idx()].get_offset();

        let new_scale = if wheel_event.delta_y() < 0. {
            (old_draw_scale * (1.0 + zoom_factor)).min(10.0) // Zoom in
        } else {
            (old_draw_scale / (1.0 + zoom_factor)).max(0.5) // Zoom out
        };
        avb.canvases[CanvasKind::Draw.idx()].set_scale(new_scale);

        let canvas_pos = avb.canvases[CanvasKind::Draw.idx()]
            .get_user_ui()
            .canvas_pos;
        let old_draw_offset_rel = canvas_pos - old_draw_offset;
        let new_draw_offset = canvas_pos - old_draw_offset_rel * (new_scale / old_draw_scale);

        avb.canvases[CanvasKind::Draw.idx()].set_offset(new_draw_offset);
        drop(avb);
        draw_grid_and_rules(av.clone());
        render_draw_view(av);
    }
}
fn on_draw_mouse_enter(av: RefAV, _event: Event) {
    let mut avb = av.borrow_mut();
    avb.canvases[CanvasKind::Draw.idx()].set_pointer_on_canvas(true);
    let curr_pos = avb.canvases[CanvasKind::Draw.idx()]
        .get_user_ui()
        .pointer
        .curr;
    avb.canvases[CanvasKind::Draw.idx()]
        .get_user_ui_mut()
        .pointer
        .saved = curr_pos;
    // if let Some(elem) = &mut avb.element_on_creation {
    //     elem.move_drawable(curr_pos);
    //     elem.save_vertices_positions();
    // }
    drop(avb);
    render_draw_view(av);
}
fn on_draw_mouse_leave(av: RefAV, _event: Event) {
    let mut avb = av.borrow_mut();
    avb.canvases[CanvasKind::Draw.idx()].set_pointer_on_canvas(false);
    avb.element_on_creation = None;
    avb.go_to_arrow_tool();
    drop(avb);
    render_draw_view(av);
}
fn on_draw_context_menu(_av: RefAV, event: Event) {
    // Prevent the default context menu from appearing
    event.prevent_default();
}
fn on_gcode_context_menu(_av: RefAV, event: Event) {
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

fn on_gcode_mouse_move(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.clone().dyn_into::<MouseEvent>() {
        let ua = av
            .borrow_mut()
            .update_canvas_inputs(mouse_event, SystemMouse::Move);
        let mut avb = av.borrow_mut();
        if let UserAction::Move(MouseButton::Right, ButtonLevel::Down) = ua {
            avb.canvases[CanvasKind::Gcode.idx()].move_offset();
        }
        drop(avb);
    }
    render_gcode_view(av);
}
fn on_gcode_mouse_down(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.clone().dyn_into::<MouseEvent>() {
        let clicks = mouse_event.detail();
        let ua = av
            .borrow_mut()
            .update_canvas_inputs(mouse_event, SystemMouse::Down(clicks));
        // Hide the context menu when clicking elsewhere
        display_html_element(DOMElements::ContextMenuShape, false);
        let mut avb = av.borrow_mut();
        if let UserAction::ClickDown(MouseButton::Right, _) = ua {
            avb.canvases[CanvasKind::Gcode.idx()].save_offset();
        }
        drop(avb);
    }
    // update_status_bar(&mut pam);
    render_gcode_view(av);
}
fn on_gcode_mouse_up(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.clone().dyn_into::<MouseEvent>() {
        let clicks = mouse_event.detail();
        let ua = av
            .borrow_mut()
            .update_canvas_inputs(mouse_event, SystemMouse::Up(clicks));
        drop(ua);
    }
    render_gcode_view(av);
}
fn on_gcode_mouse_wheel(av: RefAV, event: Event) {
    if let Ok(wheel_event) = event.dyn_into::<WheelEvent>() {
        wheel_event.prevent_default();
        let mut avb = av.borrow_mut();

        let zoom_factor = 0.04;
        let old_draw_scale = avb.canvases[CanvasKind::Gcode.idx()].get_scale();
        let old_draw_offset = avb.canvases[CanvasKind::Gcode.idx()].get_offset();

        let new_scale = if wheel_event.delta_y() < 0. {
            (old_draw_scale * (1.0 + zoom_factor)).min(10.0) // Zoom in
        } else {
            (old_draw_scale / (1.0 + zoom_factor)).max(0.5) // Zoom out
        };
        avb.canvases[CanvasKind::Gcode.idx()].set_scale(new_scale);

        let canvas_pos = avb.canvases[CanvasKind::Gcode.idx()]
            .get_user_ui()
            .canvas_pos;
        let old_draw_offset_rel = canvas_pos - old_draw_offset;
        let new_draw_offset = canvas_pos - old_draw_offset_rel * (new_scale / old_draw_scale);

        avb.canvases[CanvasKind::Gcode.idx()].set_offset(new_draw_offset);

        drop(avb);
        draw_grid_and_rules(av.clone());
        render_gcode_view(av);
    }
}

///////////////
// Window events
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

fn resize_canvases(av: RefAV) {
    log!("resize all canvases");
    let mut avb = av.borrow_mut();
    let active_canvas = avb.active_canvas;

    let window_width = avb.window.inner_width().unwrap().as_f64().unwrap() as u32;
    let window_height = avb.window.inner_height().unwrap().as_f64().unwrap() as u32;

    let offset_start_x = get_element_width(&avb.left_panel);
    let offset_start_y = get_element_height(&avb.top_menu);

    let main_w = window_width - offset_start_x;
    let main_h = window_height - offset_start_y;

    // Canvas de dessin (stack)
    avb.canvases[CanvasKind::Background.idx()].resize(main_w, main_h);
    avb.canvases[CanvasKind::Grid.idx()].resize(main_w, main_h);
    avb.canvases[CanvasKind::Draw.idx()].resize(main_w, main_h);

    // Canvas G-code (panneau gauche du split)
    if active_canvas == CanvasKind::Gcode {
        if let Some(left) = avb.document.get_element_by_id("gcode-left") {
            let rect = left.get_bounding_client_rect();
            let gw = rect.width().max(1.0) as u32;
            let gh = rect.height().max(1.0) as u32;
            avb.canvases[CanvasKind::Gcode.idx()].resize(gw, gh);
        }
    }

    drop(avb);

    // Redraw selon vue
    draw_grid_and_rules(av.clone());
    if active_canvas == CanvasKind::Gcode {
        render_gcode_view(av);
    } else {
        render_draw_view(av);
    }
}
fn on_window_resize(av: RefAV, _event: Event) {
    resize_canvases(av.clone());
    render_draw_view(av);
}
fn on_window_click(_pa: RefAV, _event: Event) {}
fn on_window_keydown(av: RefAV, event: Event) {
    event.prevent_default();
    if let Ok(keyboard_event) = event.dyn_into::<KeyboardEvent>() {
        if let Some(key) = Keys::from_str(&keyboard_event.key()).ok() {
            let mut avb = av.borrow_mut();

            let canvas = avb.get_user_canvas_mut();
            use Keys::*;
            match key {
                Control | Meta => {
                    log!("control pressed");
                    canvas.get_user_ui_mut().keys_states.ctrl_cmd_pressed = true;
                }
                Shift => {
                    log!("shift pressed");
                    canvas.get_user_ui_mut().keys_states.shift_pressed = true;
                }
                Alt => {
                    log!("alt pressed");
                    canvas.get_user_ui_mut().keys_states.alt_pressed = true;
                }
                Delete | Backspace => avb.del_back_pressed(),
                Escape => avb.esc_pressed(),
                // Copy and paste
                CLower => {
                    if canvas.get_user_ui().keys_states.ctrl_cmd_pressed {
                        log!("ctrl-c pressed");
                        avb.ctrl_c_pressed();
                    }
                }
                VLower => {
                    if canvas.get_user_ui().keys_states.ctrl_cmd_pressed {
                        log!("ctrl-v pressed");
                        avb.ctrl_v_pressed();
                    }
                }

                // Undo and Redo
                ZLower => {
                    if canvas.get_user_ui().keys_states.ctrl_cmd_pressed {
                        log!("ctrl-z pressed");
                        avb.undo();
                    }
                }
                ZUpper | YLower => {
                    if canvas.get_user_ui().keys_states.ctrl_cmd_pressed {
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
                    cnc_send(av.clone(), cmd);
                    drop(avb);
                    render_draw_view(av);
                    return;
                }
                ArrowRight => {
                    // Jog X +1mm
                    let cmd = "$J=G91 X1 F200".to_string();
                    cnc_send(av.clone(), cmd);
                    drop(avb);
                    render_draw_view(av);
                    return;
                }
                _ => (),
            };
            drop(avb);
            render_draw_view(av);
        }
    }
}
fn on_window_keyup(av: RefAV, event: Event) {
    if let Ok(keyboard_event) = event.dyn_into::<KeyboardEvent>() {
        if let Some(key) = Keys::from_str(&keyboard_event.key()).ok() {
            let mut avb = av.borrow_mut();
            let canvas = avb.get_user_canvas_mut();
            use Keys::*;
            match key {
                Control | Meta => {
                    log!("control released");
                    canvas.get_user_ui_mut().keys_states.ctrl_cmd_pressed = false;
                }
                Shift => {
                    log!("shift released");
                    canvas.get_user_ui_mut().keys_states.shift_pressed = false;
                }
                Alt => {
                    log!("alt released");
                    canvas.get_user_ui_mut().keys_states.alt_pressed = false;
                }
                _ => (),
            }
            drop(avb);
            render_draw_view(av);
        }
    }
}
///////////////
// Icons events
fn on_tab_click(av: RefAV, selected: Tabs) {
    let mut avb = av.borrow_mut();

    // UI: active tab + view
    for t in [Tabs::Draw, Tabs::Gcode] {
        if let Some(btn) = t.get_element() {
            let cl = btn.class_list();
            let _ = if t == selected {
                cl.add_1("active")
            } else {
                cl.remove_1("active")
            };
        }
        if let Some(view) = document().get_element_by_id(t.view_id()) {
            let cl = view.class_list();
            let _ = if t == selected {
                cl.add_1("active")
            } else {
                cl.remove_1("active")
            };
        }
    }
    // Mode panneau gauche
    if let Some(left_panel) = avb.document.get_element_by_id("left-panel") {
        let cl = left_panel.class_list();
        match selected {
            Tabs::Gcode => {
                let _ = cl.add_1("gcode-mode");
            }
            Tabs::Draw => {
                let _ = cl.remove_1("gcode-mode");
            }
        }
    }

    avb.active_view = selected;

    // Recalculer le g-code
    match avb.active_view {
        Tabs::Draw => {
            avb.active_canvas = CanvasKind::Draw;
        }
        Tabs::Gcode => {
            avb.active_canvas = CanvasKind::Gcode;
        }
    }

    if let Tabs::Gcode = selected {
        let params = CutParams {
            feed_xy: 1200.0,
            pierce_delay_s: 0.3,
            torch_on_m3: "M3",
            torch_off_m5: "M5",
        };

        let gcode = multipolygon_to_plasma_gcode(
            &avb.canvases[CanvasKind::Draw.idx()].dataset.final_polygon,
            &params,
        );
        avb.last_gcode = Some(gcode);
        avb.gcode_auto_center = true;
        avb.gcode_auto_fit = true;
    }

    drop(avb);

    resize_canvases(av.clone());
}
fn on_icon_click(av: RefAV, event: Event) {
    let mut avb = av.borrow_mut();
    if let Some(target) = event.target() {
        if let Some(element) = wasm_bindgen::JsCast::dyn_ref::<Element>(&target) {
            if let Some(id) = element.get_attribute("id") {
                if let Some(icon) = avb.user_icons.iter().find(|&&k| k.id() == id).cloned() {
                    avb.icon_selected = icon;
                    avb.get_user_canvas_mut().dataset.shapes_highlighted.clear();
                    avb.get_user_canvas_mut().dataset.shapes_selected.clear();
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
    render_draw_view(av);
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
fn draw_reset_origin(av: RefAV) {
    av.borrow_mut().get_user_canvas_mut().reset_origin();
}

fn draw_grid_and_rules(av: RefAV) {
    av.borrow_mut().get_user_canvas_mut().draw_origin();
}

fn render_draw_informations(av: RefAV) {
    let mut avb = av.borrow_mut();
    let canvas_draw = &mut avb.canvases[CanvasKind::Draw.idx()];
    let c_size = canvas_draw.get_canvas_size();
    let pos = canvas_draw.get_user_ui().pointer.curr;
    canvas_draw.direct_text(&CanvasText::new(
        format!("({:.2},{:.2})", pos.x, pos.y),
        TextPos::PosCustom(Vec2::new(c_size.width - 200., c_size.height - 10.)),
        CanvasTextConfig::new(Color::Rules, 0., TextAlign::Left, 20, 0.4),
    ));
    canvas_draw.direct_text(&CanvasText::new(
        format!(
            "Snap linear: {:.0} mm",
            canvas_draw.get_user_ui().snap.linear(),
        ),
        TextPos::PosCustom(Vec2::new(0., c_size.height - 30.)),
        CanvasTextConfig::new(Color::Rules, 0., TextAlign::Left, 20, 0.4),
    ));
    canvas_draw.direct_text(&CanvasText::new(
        format!("Snap angle: {:.0}°", canvas_draw.get_user_ui().snap.angle()),
        TextPos::PosCustom(Vec2::new(0., c_size.height - 10.)),
        CanvasTextConfig::new(Color::Rules, 0., TextAlign::Left, 20, 0.4),
    ));
}

fn render_draw_view(av: RefAV) {
    let mut avb = av.borrow_mut();
    let element_on_creation = avb.element_on_creation.clone();
    let canvas_draw = &mut avb.canvases[CanvasKind::Draw.idx()];
    // use IconsShapes::*;
    // Get the Performance API
    // let performance = window().unwrap().performance().unwrap();
    // let start_time = performance.now();

    canvas_draw.clear();

    // Draw full contour
    canvas_draw.draw_closed_path(Pattern::Composed(true), Color::Black, Color::Gray20, vec![]);

    if !canvas_draw.get_user_ui().keys_states.alt_pressed {
        // Draw the elements (shapes outlines)
        canvas_draw.draw_paths_sets();

        // Draw radiuses -if any- for square and polygon
        canvas_draw.may_be_draw_radiuses();

        // Draw vertices, dimensions and get the bind on the same time
        let binds: Binding<(EUId, VUId)> = canvas_draw.draw_vertices();

        // Draw the binds dimensions
        for Couple((eid1, vid1), (eid2, vid2)) in binds.iter() {
            // Get elements e1 and e2
            if let (Some(e1), Some(e2)) = (
                canvas_draw.dataset.get_element(*eid1),
                canvas_draw.dataset.get_element(*eid2),
            ) {
                // Get vertices v1 and v2
                if let (Some(v1), Some(v2)) = (e1.get_vertex(vid1), e2.get_vertex(vid2)) {
                    // Draw the binding segment
                    if let Some(seg) = SegBundle::new(v1.curr, v2.curr) {
                        let (path, pattern, colors, text) =
                            dim_hv(seg, canvas_draw.get_canvas_infos());
                        canvas_draw.draw_path(
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

    if let Some((cs, mut vs)) = element_on_creation {
        match cs {
            Icons::Disc | Icons::Square | Icons::Oblong => {
                if vs.len() == 1 {
                    vs.push(canvas_draw.get_user_ui().pointer.curr);
                    if let Some(e) = ClosedShape::new(cs, &vs) {
                        canvas_draw.draw_paths_creation(&e);
                        canvas_draw.draw_vs(&e);
                        canvas_draw.draw_dimensions(&e);
                    }
                }
            }
            Icons::Poly => {
                if vs.len() > 2 {
                    vs.push(canvas_draw.get_user_ui().pointer.curr);
                    if let Some(e) = ClosedShape::new(cs, &vs) {
                        canvas_draw.draw_paths_creation(&e);
                        canvas_draw.draw_vs(&e);
                        canvas_draw.draw_dimensions(&e);
                    }
                } else {
                    vs.push(canvas_draw.get_user_ui().pointer.curr);
                    if vs.len() >= 2 {
                        let colors = get_vertices_colors(false, false);
                        for (i, v) in vs.iter().enumerate() {
                            if i < vs.len() - 1 {
                                // Line
                                canvas_draw.draw_path(
                                    &line_path(vs[i], vs[i + 1]),
                                    Pattern::OnCreation,
                                    colors.fill_color,
                                    colors.stroke_color,
                                    vec![],
                                );
                                // Dimension
                                if let Some(seg) = SegBundle::new(vs[i], vs[i + 1]) {
                                    let (path, pattern, colors, text) =
                                        dim_hv(seg, canvas_draw.get_canvas_infos());
                                    canvas_draw.draw_path(
                                        &path,
                                        pattern,
                                        colors.fill_color,
                                        colors.stroke_color,
                                        text,
                                    );
                                }
                            }
                            canvas_draw.draw_path(
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
    canvas_draw.draw_pointer(canvas_draw.get_user_ui().pointer.curr);

    drop(avb);
    render_draw_informations(av);
}

fn render_gcode_view(av: RefAV) {
    let mut avb = av.borrow_mut();
    let gcode: String = {
        // Ici on clone le gcode, à voir si on peut éviter ça plus tard (volume de données ?)
        avb.last_gcode.clone().unwrap_or_default()
    };

    if let Some(el) = avb.document.get_element_by_id("gcode-text") {
        let el: web_sys::HtmlElement = el.dyn_into().ok().unwrap();
        el.set_inner_text(&gcode);
    }

    let segs = gcode_to_segments(&gcode);

    // Dessine seulement les segments de coupe (torch ON)
    let gcode_action = if avb.gcode_auto_fit {
        avb.gcode_auto_center = false;
        avb.gcode_auto_fit = false;
        Some(true)
    } else if avb.gcode_auto_center {
        avb.gcode_auto_center = false;
        Some(false)
    } else {
        None
    };

    if let Some(do_fit) = gcode_action {
        let canvas_gcode = &mut avb.canvases[CanvasKind::Gcode.idx()];
        if do_fit {
            fit_gcode_canvas(canvas_gcode, &segs);
        } else {
            center_gcode_canvas(canvas_gcode, &segs);
        }
    }
    let canvas_gcode = &mut avb.canvases[CanvasKind::Gcode.idx()];
    canvas_gcode.clear();

    for s in segs.into_iter().filter(|s| s.cut) {
        let p = seg_to_path(s);
        log!("G-code path: {:?}", p);
        canvas_gcode.draw_path(&p, Pattern::Basic, Color::Transparent, Color::Black, vec![]);
    }
}

fn seg_to_path(s: Seg) -> BezPath {
    BezPath::from_vec(vec![
        PathEl::MoveTo(Point::new(s.x1, s.y1)),
        PathEl::LineTo(Point::new(s.x2, s.y2)),
    ])
}

fn center_gcode_canvas(canvas: &mut Canvas, segs: &[Seg]) {
    if segs.is_empty() {
        return;
    }

    let mut min_x = segs[0].x1.min(segs[0].x2);
    let mut max_x = segs[0].x1.max(segs[0].x2);
    let mut min_y = segs[0].y1.min(segs[0].y2);
    let mut max_y = segs[0].y1.max(segs[0].y2);

    for s in segs.iter() {
        let sx_min = s.x1.min(s.x2);
        let sx_max = s.x1.max(s.x2);
        let sy_min = s.y1.min(s.y2);
        let sy_max = s.y1.max(s.y2);
        min_x = min_x.min(sx_min);
        max_x = max_x.max(sx_max);
        min_y = min_y.min(sy_min);
        max_y = max_y.max(sy_max);
    }

    let center = Vec2::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
    let size = canvas.get_canvas_size();
    let canvas_center = Vec2::new(size.width * 0.5, size.height * 0.5);
    let offset = canvas_center - center * canvas.get_scale();
    canvas.set_offset(offset);
}

fn fit_gcode_canvas(canvas: &mut Canvas, segs: &[Seg]) {
    if segs.is_empty() {
        return;
    }

    let mut min_x = segs[0].x1.min(segs[0].x2);
    let mut max_x = segs[0].x1.max(segs[0].x2);
    let mut min_y = segs[0].y1.min(segs[0].y2);
    let mut max_y = segs[0].y1.max(segs[0].y2);

    for s in segs.iter() {
        let sx_min = s.x1.min(s.x2);
        let sx_max = s.x1.max(s.x2);
        let sy_min = s.y1.min(s.y2);
        let sy_max = s.y1.max(s.y2);
        min_x = min_x.min(sx_min);
        max_x = max_x.max(sx_max);
        min_y = min_y.min(sy_min);
        max_y = max_y.max(sy_max);
    }

    let bbox_w = (max_x - min_x).max(1.0);
    let bbox_h = (max_y - min_y).max(1.0);

    let size = canvas.get_canvas_size();
    let pad = 20.0;
    let avail_w = (size.width - pad * 2.0).max(1.0);
    let avail_h = (size.height - pad * 2.0).max(1.0);

    let scale = (avail_w / bbox_w).min(avail_h / bbox_h).clamp(0.5, 10.0);
    canvas.set_scale(scale);

    let center = Vec2::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
    let canvas_center = Vec2::new(size.width * 0.5, size.height * 0.5);
    let offset = canvas_center - center * scale;
    canvas.set_offset(offset);
}
