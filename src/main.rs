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
use crate::shapes::DataSet;
use crate::types::EUId;
use crate::types::VUId;
use canvas::{CanvasKind, CanvasText, CanvasTextConfig, Color, Pattern, TextAlign, TextPos};
use dimensions::dim_hv;
use inputs::Keys;
use inputs::SystemMouse;
use inputs::*;
use js_sys::{Array, Date, Reflect, JSON};
use kurbo::{BezPath, PathEl, Point, Shape};
use kurbo::{Size, Vec2};
use prefab::get_vertices_colors;
use prefab::point_path;
use shape::ClosedShape;
use shape::Operation;
use shape::TextData;
use shape::TextFont;
use std::collections::HashSet;
use std::str::FromStr;
use std::{cell::RefCell, rc::Rc};
use types::Binding;
use types::Couple;
use types::SegBundle;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    window, Blob, BlobPropertyBag, Document, Element, Event, FileReader, HtmlAnchorElement,
    HtmlCanvasElement, HtmlElement, HtmlInputElement, KeyboardEvent, MouseEvent, Url, WheelEvent,
    Window,
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
    fn ctrl_s_pressed(&mut self) {
        if let Some(element) = self.document.get_element_by_id("save-option") {
            if let Ok(button) = element.dyn_into::<HtmlElement>() {
                button.click();
            }
        }
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
    fn edit_text_at(&mut self, pos: Vec2) -> bool {
        let canvas = &mut self.canvases[CanvasKind::Draw.idx()];
        let shift_pressed = canvas.get_user_ui().keys_states.shift_pressed;
        for (_eid, shape) in canvas.dataset.shapes.iter_mut() {
            if shape.get_shape_type() != Icons::Text {
                continue;
            }
            if !shape.contains(pos) {
                continue;
            }
            if shift_pressed {
                let auto_fit = shape.get_text().map(|text| !text.auto_fit).unwrap_or(false);
                shape.set_text_autofit(auto_fit);
                canvas.dataset.calc_final_polygon();
                return true;
            }
            shape.ensure_text_scale();
            let current = shape
                .get_text()
                .map(|text| text.text.as_str())
                .unwrap_or("");
            let edited = self
                .window
                .prompt_with_message_and_default("Edit text", current)
                .ok()
                .flatten();
            if let Some(new_text) = edited {
                shape.set_text_value(new_text);
                shape.fit_text_bbox_width_to_content();
                canvas.dataset.calc_final_polygon();
            }
            canvas.dataset.shapes_selected.clear();
            canvas.dataset.shapes_highlighted.clear();
            canvas.dataset.vertices_selected.clear();
            canvas.dataset.vertices_highlighted.clear();
            canvas
                .dataset
                .shapes_selector
                .refresh_selectable_elems(HashSet::new());
            return true;
        }
        false
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

    fn _get_user_canvas(&self) -> &Canvas {
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
    user_icons.insert(Text);

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
    let document = av.borrow().document.clone();

    let load_element = document.get_element_by_id("load-option").unwrap();
    let load_element: HtmlElement = load_element.dyn_into::<HtmlElement>()?;

    let save_element = document.get_element_by_id("save-option").unwrap();
    let save_element: HtmlElement = save_element.dyn_into::<HtmlElement>()?;

    let export_svg_element = document.get_element_by_id("export-svg").unwrap();
    let export_svg_element: HtmlElement = export_svg_element.dyn_into::<HtmlElement>()?;

    let file_input = document.get_element_by_id("file-input").unwrap();
    let file_input: HtmlInputElement = file_input.dyn_into::<HtmlInputElement>()?;

    let file_input_clone = file_input.clone();

    let on_load = Closure::wrap(Box::new(move || {
        file_input_clone.click();
    }) as Box<dyn FnMut()>);

    load_element.add_event_listener_with_callback("click", on_load.as_ref().unchecked_ref())?;
    on_load.forget();

    let document_clone = document.clone();
    let av_clone = av.clone();
    let on_save = Closure::wrap(Box::new(move || {
        let canvas = &av_clone.borrow().canvases[CanvasKind::Draw.idx()];
        let export_info = make_export_info(&document_clone);
        let meta = ExportMeta {
            title: export_info.title.clone(),
            timestamp: export_info.timestamp.clone(),
            canvas_size: canvas.get_size(),
            canvas_scale: canvas.get_scale(),
            canvas_offset: canvas.get_offset(),
        };

        let json = build_json_from_dataset(&canvas.dataset, &meta);
        if let Some(json) = json {
            let filename = format!("{}.json", export_info.basename);
            trigger_download(&document_clone, &filename, &json, "application/json");
        }
    }) as Box<dyn FnMut()>);

    save_element.add_event_listener_with_callback("click", on_save.as_ref().unchecked_ref())?;
    on_save.forget();

    let document_clone = document.clone();
    let av_clone = av.clone();
    let on_export_svg = Closure::wrap(Box::new(move || {
        let canvas = &av_clone.borrow().canvases[CanvasKind::Draw.idx()];
        let export_info = make_export_info(&document_clone);

        let svg = build_svg_from_dataset(&canvas.dataset);
        if let Some(svg) = svg {
            let filename = format!("{}.svg", export_info.basename);
            trigger_download(&document_clone, &filename, &svg, "image/svg+xml");
        }
    }) as Box<dyn FnMut()>);

    export_svg_element
        .add_event_listener_with_callback("click", on_export_svg.as_ref().unchecked_ref())?;
    on_export_svg.forget();

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
                        load_json_to_dataset(av_clone.clone(), result);
                    }
                }) as Box<dyn FnMut(_)>);

                file_reader
                    .add_event_listener_with_callback("load", on_load.as_ref().unchecked_ref())
                    .unwrap();
                on_load.forget();

                file_reader.read_as_text(&file).unwrap();
            }
            file_input.set_value("");
        }
    }) as Box<dyn FnMut()>);

    file_input
        .add_event_listener_with_callback("change", on_file_select.as_ref().unchecked_ref())?;
    on_file_select.forget();

    Ok(())
}

fn build_svg_from_dataset(dataset: &DataSet) -> Option<String> {
    let mut paths: Vec<BezPath> = Vec::new();

    for shape in dataset.shapes.values() {
        if shape.get_shape_type() == Icons::Text {
            paths.extend(geo_multipolygon_to_bez_paths(shape.get_polygon()));
        } else if !shape.get_bezpath().is_empty() {
            paths.push(shape.get_bezpath().clone());
        }
    }

    if paths.is_empty() {
        return None;
    }

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for path in &paths {
        if path.is_empty() {
            continue;
        }
        let bbox = path.bounding_box();
        min_x = min_x.min(bbox.x0);
        min_y = min_y.min(bbox.y0);
        max_x = max_x.max(bbox.x1);
        max_y = max_y.max(bbox.y1);
    }

    if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
        return None;
    }

    let width = (max_x - min_x).max(1.0);
    let height = (max_y - min_y).max(1.0);

    let mut svg = String::new();
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\">\n",
        min_x, min_y, width, height
    ));
    svg.push_str("  <g fill=\"none\" stroke=\"black\" stroke-width=\"1\">\n");
    for path in paths {
        svg.push_str(&format!("    <path d=\"{}\" />\n", path.to_svg()));
    }
    svg.push_str("  </g>\n</svg>\n");
    Some(svg)
}

fn build_json_from_dataset(dataset: &DataSet, meta: &ExportMeta) -> Option<String> {
    if dataset.shapes.is_empty() {
        return None;
    }

    let mut out = String::from("{\"version\":1,\"meta\":{");
    out.push_str("\"title\":\"");
    out.push_str(&json_escape(&meta.title));
    out.push_str("\",\"timestamp\":\"");
    out.push_str(&json_escape(&meta.timestamp));
    out.push_str("\",\"canvas\":{");
    out.push_str(&format!(
        "\"size\":[{:.6},{:.6}],\"scale\":{:.6},\"offset\":[{:.6},{:.6}]",
        meta.canvas_size.width,
        meta.canvas_size.height,
        meta.canvas_scale,
        meta.canvas_offset.x,
        meta.canvas_offset.y
    ));
    out.push_str("}},\"shapes\":[");
    let mut first_shape = true;

    let mut shapes: Vec<_> = dataset.shapes.iter().collect();
    shapes.sort_by_key(|(id, _)| **id);
    for (eid, shape) in shapes {
        if !first_shape {
            out.push(',');
        }
        first_shape = false;

        out.push_str("{\"id\":");
        out.push_str(&format!("{eid}"));
        out.push_str(",\"type\":\"");
        out.push_str(icon_to_name(shape.get_shape_type()));
        out.push_str("\",\"operation\":\"");
        out.push_str(operation_to_name(shape.get_operation()));
        out.push_str("\",\"vertices\":[");

        let mut first_vertex = true;
        for (_, value) in shape.get_vertices().iter() {
            if !first_vertex {
                out.push(',');
            }
            first_vertex = false;
            out.push_str("{\"x\":");
            out.push_str(&format!("{:.6}", value.curr.x));
            out.push_str(",\"y\":");
            out.push_str(&format!("{:.6}", value.curr.y));
            out.push_str(",\"rounded\":");
            match value.rounded {
                Some(rounded) => out.push_str(&rounded.to_string()),
                None => out.push_str("null"),
            }
            out.push('}');
        }
        out.push(']');

        if let Some(text) = shape.get_text() {
            out.push_str(",\"text\":{\"value\":\"");
            out.push_str(&json_escape(&text.text));
            out.push_str("\",\"font\":\"");
            out.push_str(text_font_to_name(&text.font));
            out.push_str("\",\"scale\":");
            match text.scale {
                Some(scale) => out.push_str(&format!("{:.6}", scale)),
                None => out.push_str("null"),
            }
            out.push_str(",\"auto_fit\":");
            out.push_str(if text.auto_fit { "true" } else { "false" });
            out.push('}');
        }

        out.push('}');
    }

    out.push_str("]}");
    Some(out)
}

fn get_prop(value: &JsValue, name: &str) -> Option<JsValue> {
    Reflect::get(value, &JsValue::from_str(name)).ok()
}

fn get_string(value: &JsValue, name: &str) -> Option<String> {
    get_prop(value, name).and_then(|val| val.as_string())
}

fn get_f64(value: &JsValue, name: &str) -> Option<f64> {
    get_prop(value, name).and_then(|val| val.as_f64())
}

fn get_vec2_array(value: &JsValue, name: &str) -> Option<Vec2> {
    let arr_value = get_prop(value, name)?;
    let arr = Array::from(&arr_value);
    if arr.length() < 2 {
        return None;
    }
    let x = arr.get(0).as_f64()?;
    let y = arr.get(1).as_f64()?;
    Some(Vec2::new(x, y))
}

fn name_to_icon(name: &str) -> Option<Icons> {
    match name {
        "arrow" => Some(Icons::Arrow),
        "disc" => Some(Icons::Disc),
        "square" => Some(Icons::Square),
        "oblong" => Some(Icons::Oblong),
        "poly" => Some(Icons::Poly),
        "text" => Some(Icons::Text),
        _ => None,
    }
}

fn name_to_operation(name: String) -> Option<Operation> {
    match name.as_str() {
        "union" => Some(Operation::Union),
        "difference" => Some(Operation::Difference),
        _ => None,
    }
}

fn name_to_text_font(name: String) -> Option<TextFont> {
    match name.as_str() {
        "stencilia" => Some(TextFont::Stencilia),
        "urbanist" => Some(TextFont::Urbanist),
        _ => None,
    }
}

fn trigger_download(document: &Document, filename: &str, contents: &str, mime: &str) {
    let parts = Array::new();
    parts.push(&JsValue::from_str(contents));

    let options = BlobPropertyBag::new();
    options.set_type(mime);

    let blob = match Blob::new_with_str_sequence_and_options(&parts, &options) {
        Ok(blob) => blob,
        Err(_) => return,
    };
    let url = match Url::create_object_url_with_blob(&blob) {
        Ok(url) => url,
        Err(_) => return,
    };

    if let Some(body) = document.body() {
        if let Ok(el) = document.create_element("a") {
            if let Ok(link) = el.dyn_into::<HtmlAnchorElement>() {
                link.set_href(&url);
                link.set_download(filename);
                let _ = body.append_child(&link);
                link.click();
                let _ = body.remove_child(&link);
            }
        }
    }
    let _ = Url::revoke_object_url(&url);
}

struct ExportMeta {
    title: String,
    timestamp: String,
    canvas_size: Size,
    canvas_scale: f64,
    canvas_offset: Vec2,
}

struct ExportInfo {
    title: String,
    timestamp: String,
    basename: String,
}

fn make_export_info(document: &Document) -> ExportInfo {
    let raw_title = document.title();
    let title = if raw_title.trim().is_empty() {
        "drawing".to_string()
    } else {
        raw_title
    };
    let timestamp = timestamp_string();
    let base = sanitize_filename(&title);
    let basename = if base.is_empty() {
        format!("drawing-{}", timestamp)
    } else {
        format!("{}-{}", base, timestamp)
    };
    ExportInfo {
        title,
        timestamp,
        basename,
    }
}

fn sanitize_filename(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in input.chars() {
        let c = if ch.is_ascii_alphanumeric() {
            Some(ch.to_ascii_lowercase())
        } else if ch == ' ' || ch == '-' || ch == '_' {
            Some('-')
        } else {
            None
        };
        if let Some(c) = c {
            if c == '-' {
                if !last_dash && !out.is_empty() {
                    out.push(c);
                    last_dash = true;
                }
            } else {
                out.push(c);
                last_dash = false;
            }
        }
    }
    if out.ends_with('-') {
        out.pop();
    }
    out
}

fn timestamp_string() -> String {
    let date = Date::new_0();
    let year = date.get_full_year() as i32;
    let month = (date.get_month() + 1) as i32;
    let day = date.get_date() as i32;
    let hours = date.get_hours() as i32;
    let minutes = date.get_minutes() as i32;
    let seconds = date.get_seconds() as i32;
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        year, month, day, hours, minutes, seconds
    )
}

fn icon_to_name(icon: Icons) -> &'static str {
    match icon {
        Icons::Arrow => "arrow",
        Icons::Disc => "disc",
        Icons::Square => "square",
        Icons::Oblong => "oblong",
        Icons::Poly => "poly",
        Icons::Text => "text",
    }
}

fn operation_to_name(operation: Operation) -> &'static str {
    match operation {
        Operation::Union => "union",
        Operation::Difference => "difference",
    }
}

fn text_font_to_name(font: &TextFont) -> &'static str {
    match font {
        TextFont::Stencilia => "stencilia",
        TextFont::Urbanist => "urbanist",
    }
}

fn json_escape(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
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

fn load_json_to_dataset(av: RefAV, json_data: String) {
    let Ok(value) = JSON::parse(&json_data) else {
        log!("Invalid JSON file.");
        return;
    };
    let Some(shapes_value) = get_prop(&value, "shapes") else {
        log!("Missing shapes array.");
        return;
    };

    let shapes_array = Array::from(&shapes_value);
    if shapes_array.length() == 0 {
        log!("No shapes in file.");
        return;
    }

    let mut avb = av.borrow_mut();
    let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
    canvas.dataset = DataSet::new();

    if let Some(meta_value) = get_prop(&value, "meta") {
        if let Some(canvas_value) = get_prop(&meta_value, "canvas") {
            if let Some(scale) = get_f64(&canvas_value, "scale") {
                canvas.set_scale(scale);
            }
            if let Some(offset) = get_vec2_array(&canvas_value, "offset") {
                canvas.set_offset(offset);
            }
        }
    }

    for shape_value in shapes_array.iter() {
        let Some(shape_type) =
            get_string(&shape_value, "type").and_then(|name| name_to_icon(&name))
        else {
            continue;
        };
        let operation = get_string(&shape_value, "operation")
            .and_then(name_to_operation)
            .unwrap_or(Operation::Union);

        let Some(vertices_value) = get_prop(&shape_value, "vertices") else {
            continue;
        };
        let vertices_array = Array::from(&vertices_value);
        if vertices_array.length() == 0 {
            continue;
        }

        let mut vertices = Vec::new();
        let mut rounded = Vec::new();
        for vertex_value in vertices_array.iter() {
            let Some(x) = get_f64(&vertex_value, "x") else {
                continue;
            };
            let Some(y) = get_f64(&vertex_value, "y") else {
                continue;
            };
            vertices.push(Vec2::new(x, y));
            let rounded_value = get_prop(&vertex_value, "rounded")
                .and_then(|val| val.as_f64())
                .map(|val| val as u32);
            rounded.push(rounded_value);
        }

        let text_data = if shape_type == Icons::Text {
            let text_value = get_prop(&shape_value, "text")
                .and_then(|val| get_string(&val, "value"))
                .unwrap_or_else(|| "TEXT".to_string());
            let font_value = get_prop(&shape_value, "text")
                .and_then(|val| get_string(&val, "font"))
                .and_then(name_to_text_font)
                .unwrap_or(TextFont::Stencilia);
            let scale_value = get_prop(&shape_value, "text").and_then(|val| get_f64(&val, "scale"));
            let auto_fit = get_prop(&shape_value, "text")
                .and_then(|val| get_prop(&val, "auto_fit"))
                .and_then(|val| val.as_bool())
                .unwrap_or(false);
            Some(TextData::new(text_value, font_value, scale_value, auto_fit))
        } else {
            None
        };

        let Some(shape) =
            ClosedShape::from_raw(shape_type, operation, vertices, &rounded, text_data)
        else {
            continue;
        };

        canvas.dataset.push_element(shape);
    }

    canvas.dataset.calc_final_polygon();

    drop(avb);
    render_draw_view(av.clone());
}

fn update(av: RefAV, user_action: UserAction) -> Result<(), MyError> {
    let mut avb = av.borrow_mut();
    match avb.icon_selected {
        Icons::Arrow => match user_action {
            UserAction::ClickDown(button, clicks) => {
                if button == MouseButton::Left {
                    if clicks == 2 {
                        let draw_pos = avb.canvases[CanvasKind::Draw.idx()].get_user_ui().draw_pos;
                        if avb.edit_text_at(draw_pos) {
                            return Ok(());
                        }
                    }
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
        Icons::Disc | Icons::Square | Icons::Oblong | Icons::Poly | Icons::Text => {
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
                                Icons::Text => {
                                    if vs.is_empty() {
                                        vs.push(pointer_pos);
                                        avb.element_on_creation = Some((cs, vs));
                                    } else {
                                        vs.push(pointer_pos);
                                        if let Some(e) = ClosedShape::new_text(
                                            "TEXT".to_string(),
                                            TextFont::Stencilia,
                                            vs[0],
                                            vs[1],
                                        ) {
                                            avb.canvases[CanvasKind::Draw.idx()]
                                                .dataset
                                                .push_element(e);
                                            avb.element_on_creation = Some((cs, vec![]));
                                        } else {
                                            log!("Error creating text element");
                                        }
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
                                    Icons::Text => {
                                        if vs.is_empty() {
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
                    drop(avb);
                    cnc_send(av.clone(), cmd);
                    render_draw_view(av);
                    return;
                }
                ArrowRight => {
                    // Jog X +1mm
                    let cmd = "$J=G91 X1 F200".to_string();
                    drop(avb);
                    cnc_send(av.clone(), cmd);

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
            Icons::Text => {
                if vs.len() == 1 {
                    vs.push(canvas_draw.get_user_ui().pointer.curr);
                    if let Some(e) =
                        ClosedShape::new_text("TEXT".to_string(), TextFont::Stencilia, vs[0], vs[1])
                    {
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
