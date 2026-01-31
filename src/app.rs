use crate::canvas::{Canvas, CanvasKind};
use crate::dom::{
    get_element_height, get_element_width, init_gcode_splitter, init_menu, init_status, init_tabs,
    init_toolpath_splitter, init_window, Tabs,
};
use crate::import_export::{get_prop, load_json_to_dataset};
use crate::inputs::{SystemMouse, UserAction};
use crate::machine::{update_machine_value, MachineGroup};
use crate::shape::ShapeType;
use crate::shapes::{Toolpath, ToolpathParams};
use crate::status::update_status_bar;
use crate::ui::resize_canvases;
use crate::view_draw::app::{
    draw_grid_and_rules, draw_reset_origin, init_draw_canvas, init_icons, init_shapes_panel,
    render_draw_view,
};
use crate::view_draw::notes_dom::{init_note_handlers, update_notes_view};
use crate::view_gcode::app::init_gcode_canvas;
use crate::view_gcode::app::Seg;
use crate::view_machine::cnc_link::CncLink;
use crate::view_toolpath::app::{
    init_toolpath_canvas, init_toolpath_panel, update_toolpath_params,
};
use js_sys::{Array, Object, Reflect, JSON};
use kurbo::{BezPath, Size, Vec2};
use std::collections::{HashMap, HashSet};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{
    Blob, Document, Element, Event, FileReader, HtmlAnchorElement, HtmlCanvasElement, HtmlElement,
    HtmlInputElement, MouseEvent, ResizeObserver, Url, Window,
};

pub(crate) type RefAV = Rc<RefCell<AppVars>>;

pub(crate) fn with_av_mut<R>(av: &RefAV, f: impl FnOnce(&mut AppVars) -> R) -> R {
    let mut avb = av.borrow_mut();
    f(&mut avb)
}

pub(crate) fn with_av_try_mut<R>(av: &RefAV, f: impl FnOnce(&mut AppVars) -> R) -> Option<R> {
    av.try_borrow_mut().ok().map(|mut avb| f(&mut avb))
}

#[allow(dead_code)]
pub(crate) struct AppVars {
    pub(crate) element_on_creation: Option<(ShapeType, Vec<Vec2>)>,
    pub(crate) render_start: Option<(&'static str, f64)>,
    pub(crate) render_notice: Option<(String, f64)>,

    // DOM
    pub(crate) window: Window,
    pub(crate) document: Document,
    pub(crate) top_menu: HtmlElement,
    pub(crate) left_panel: HtmlElement,
    pub(crate) shapes_panel: HtmlElement,
    pub(crate) user_icons: HashSet<ShapeType>,
    pub(crate) tooltip: HtmlElement,
    pub(crate) icon_selected: ShapeType,
    pub(crate) canvases: [Canvas; CanvasKind::COUNT],
    pub(crate) active_canvas: CanvasKind,
    pub(crate) active_view: Tabs,

    pub(crate) last_gcode: Option<String>,
    pub(crate) gcode_auto_center: bool,
    pub(crate) gcode_auto_fit: bool,
    pub(crate) gcode_segments: Vec<Seg>,
    pub(crate) gcode_cut_path: Option<BezPath>,
    pub(crate) toolpath: Option<Toolpath>,
    pub(crate) toolpath_params: ToolpathParams,
    pub(crate) toolpath_paths: Vec<BezPath>,
    pub(crate) toolpath_lead_ins: Vec<BezPath>,
    pub(crate) toolpath_lead_outs: Vec<BezPath>,
    pub(crate) toolpath_travels: Vec<BezPath>,
    pub(crate) toolpath_travel_arrows: Vec<BezPath>,
    pub(crate) toolpath_arrows: Vec<BezPath>,
    pub(crate) toolpath_starts: Vec<Vec2>,
    pub(crate) toolpath_ends: Vec<Vec2>,
    pub(crate) toolpath_original_paths: Vec<BezPath>,
    pub(crate) toolpath_auto_center: bool,
    pub(crate) toolpath_auto_fit: bool,
    pub(crate) machine_groups: Vec<MachineGroup>,
    pub(crate) machine_view_built: bool,
    pub(crate) machine_last_update: Option<String>,
    pub(crate) ws_connected: bool,
    pub(crate) ws_status: String,
    pub(crate) last_ws_error: Option<String>,
    pub(crate) last_http_error: Option<String>,
    pub(crate) cnc: Option<Rc<CncLink>>,
    pub(crate) shapes_drag_from: Option<usize>,
    pub(crate) note_mode: bool,
    pub(crate) note_draft: Option<NoteDraft>,
    pub(crate) note_drag: Option<NoteDrag>,
    pub(crate) note_selected: Option<usize>,
    pub(crate) notes_dom: HashMap<usize, HtmlElement>,
    pub(crate) notes_resize_observers: HashMap<usize, ResizeObserver>,
}

#[derive(Debug, Clone)]
pub(crate) struct NoteDraft {
    pub(crate) id: usize,
    pub(crate) start: Vec2,
}

#[derive(Debug, Clone)]
pub(crate) struct NoteDrag {
    pub(crate) id: usize,
    pub(crate) offset: Vec2,
}

impl AppVars {
    pub(crate) fn get_active_canvas(&self) -> &Canvas {
        use CanvasKind::*;
        match self.active_canvas {
            Gcode => &self.canvases[Gcode.idx()],
            Draw => &self.canvases[Draw.idx()],
            Background => &self.canvases[Draw.idx()],
            Grid => &self.canvases[Draw.idx()],
            Toolpath => &self.canvases[Toolpath.idx()],
        }
    }
    pub(crate) fn get_active_canvas_mut(&mut self) -> &mut Canvas {
        use CanvasKind::*;
        match self.active_canvas {
            Gcode => &mut self.canvases[Gcode.idx()],
            Draw => &mut self.canvases[Draw.idx()],
            Background => &mut self.canvases[Draw.idx()],
            Grid => &mut self.canvases[Draw.idx()],
            Toolpath => &mut self.canvases[Toolpath.idx()],
        }
    }
    pub(crate) fn update_canvas_inputs(
        &mut self,
        mouse_event: MouseEvent,
        sys_mouse: SystemMouse,
    ) -> UserAction {
        let mut origin_x = get_element_width(&self.left_panel) as f64;
        if matches!(self.active_view, Tabs::Draw) {
            origin_x += get_element_width(&self.shapes_panel) as f64;
        }
        let c_draw_origin = Vec2::new(origin_x, get_element_height(&self.top_menu) as f64);
        let action = self.canvases[self.active_canvas.idx()].update_ui(
            c_draw_origin,
            &mouse_event,
            sys_mouse,
        );
        self.snap_draw_to_constr_circle_vertices();
        action
    }
}

pub(crate) fn create_app_vars(window: Window) -> Result<(), JsValue> {
    use ShapeType::*;
    log!("Creating application variables");
    log!("Initializing icons");
    let document = window.document().expect("should have a document on window");
    let c_draw: HtmlCanvasElement = init_element!(document, "mainCanvas", HtmlCanvasElement);
    let c_grid: HtmlCanvasElement = init_element!(document, "gridCanvas", HtmlCanvasElement);
    let c_back: HtmlCanvasElement = init_element!(document, "backgroundCanvas", HtmlCanvasElement);
    let c_gcode: HtmlCanvasElement = init_element!(document, "gcodeCanvas", HtmlCanvasElement);
    let c_toolpath: HtmlCanvasElement =
        init_element!(document, "toolpathCanvas", HtmlCanvasElement);
    let tooltip: HtmlElement = init_element!(document, "tooltip", HtmlElement);
    let left_panel: HtmlElement = init_element!(document, "left-panel", HtmlElement);
    let shapes_panel: HtmlElement = init_element!(document, "shapes-panel", HtmlElement);
    let top_menu: HtmlElement = init_element!(document, "top-menu", HtmlElement);
    let canvases: [Canvas; CanvasKind::COUNT] = [
        Canvas::new(c_back, Size::new(3000., 1500.))?, // Background
        Canvas::new(c_grid, Size::new(3000., 1500.))?, // Grid
        Canvas::new(c_draw, Size::new(3000., 1500.))?, // Draw
        Canvas::new(c_gcode, Size::new(3000., 1500.))?, // Gcode
        Canvas::new(c_toolpath, Size::new(3000., 1500.))?, // Toolpath
    ];
    let active_canvas = CanvasKind::Draw;
    let mut user_icons: HashSet<ShapeType> = HashSet::new();
    user_icons.insert(Arrow);
    user_icons.insert(Disc);
    user_icons.insert(Square);
    user_icons.insert(Oblong);
    user_icons.insert(Poly);
    user_icons.insert(Text);
    user_icons.insert(Voronoi);
    user_icons.insert(ConstrLine);
    user_icons.insert(ConstrCircle);

    let cnc: Option<Rc<CncLink>> = CncLink::connect("http://192.168.1.36", "ws://192.168.1.36:81/")
        .ok()
        .map(Rc::new);

    let app_vars = Rc::new(RefCell::new(AppVars {
        element_on_creation: None,
        render_start: None,
        render_notice: None,
        window,
        document,
        top_menu,
        left_panel,
        shapes_panel,
        canvases,
        active_canvas,
        active_view: Tabs::Draw,
        user_icons,
        tooltip,
        icon_selected: ShapeType::Arrow,
        last_gcode: None,
        gcode_auto_center: false,
        gcode_auto_fit: false,
        gcode_segments: Vec::new(),
        gcode_cut_path: None,
        toolpath: None,
        toolpath_params: ToolpathParams {
            feed_xy: 1200.0,
            travel_feed_xy: 3000.0,
            pierce_delay_s: 0.3,
            lead_in_mm: 2.0,
            lead_out_mm: 2.0,
            kerf_mm: 1.0,
            torch_on_m3: "M3".to_string(),
            torch_off_m5: "M5".to_string(),
        },
        toolpath_paths: Vec::new(),
        toolpath_lead_ins: Vec::new(),
        toolpath_lead_outs: Vec::new(),
        toolpath_travels: Vec::new(),
        toolpath_travel_arrows: Vec::new(),
        toolpath_arrows: Vec::new(),
        toolpath_starts: Vec::new(),
        toolpath_ends: Vec::new(),
        toolpath_original_paths: Vec::new(),
        toolpath_auto_center: false,
        toolpath_auto_fit: false,
        machine_groups: Vec::new(),
        machine_view_built: false,
        machine_last_update: None,
        ws_connected: false,
        ws_status: "WS disconnected".to_string(),
        last_ws_error: None,
        last_http_error: None,
        cnc,
        shapes_drag_from: None,
        note_mode: false,
        note_draft: None,
        note_drag: None,
        note_selected: None,
        notes_dom: HashMap::new(),
        notes_resize_observers: HashMap::new(),
    }));

    if let Some(cnc) = app_vars.borrow().cnc.clone() {
        let av_clone = app_vars.clone();
        cnc.set_on_text_handler(Some(Box::new(move |msg| {
            if let Ok(mut avb) = av_clone.try_borrow_mut() {
                avb.handle_ws_text(&msg);
            }
        })));
        let av_clone = app_vars.clone();
        cnc.set_on_status_handler(Some(Box::new(move |msg| {
            if let Ok(mut avb) = av_clone.try_borrow_mut() {
                avb.ws_status = msg.clone();
                avb.ws_connected = msg.starts_with("WS connected");
                if msg.starts_with("WS error") || msg.starts_with("WS closed") {
                    avb.last_ws_error = Some(msg);
                } else {
                    avb.last_ws_error = None;
                }
                avb.set_machine_status_error();
            }
        })));
    }

    init_menu(app_vars.clone())?;
    init_tabs(app_vars.clone())?;
    init_gcode_splitter(app_vars.clone())?;
    init_toolpath_splitter(app_vars.clone())?;
    init_icons(app_vars.clone())?;
    init_status(app_vars.clone())?;
    init_shapes_panel(app_vars.clone())?;
    init_draw_canvas(app_vars.clone())?;
    init_note_handlers(app_vars.clone())?;
    init_gcode_canvas(app_vars.clone())?;
    init_toolpath_canvas(app_vars.clone())?;
    init_toolpath_panel(app_vars.clone())?;
    init_window(app_vars.clone())?;
    resize_canvases(app_vars.clone());

    let av = app_vars;

    if let Some(panel) = av.borrow().document.get_element_by_id("left-panel") {
        let _ = panel.class_list().add_1("draw-mode");
    }

    draw_reset_origin(av.clone());
    draw_grid_and_rules(av.clone());
    if let Some((_, data)) = crate::examples_gen::EXAMPLES
        .iter()
        .find(|(name, _)| *name == "start")
    {
        load_json_to_dataset(av.clone(), (*data).to_string());
        update_notes_view(av.clone());
    }
    update_status_bar(av.clone());
    render_draw_view(av.clone());

    Ok(())
}

pub(crate) fn save_toolpath_params(av: RefAV) {
    let document = av.borrow().document.clone();
    let (params, machine_values) = {
        let avb = av.borrow();
        let params = avb.toolpath_params.clone();
        let mut machine_values = Vec::new();
        for group in avb.machine_groups.iter() {
            for setting in group.settings.iter() {
                machine_values.push((setting.id.clone(), setting.value.clone()));
            }
        }
        (params, machine_values)
    };
    let payload = Object::new();
    let toolpath = Object::new();
    let _ = Reflect::set(
        &toolpath,
        &"feed_xy".into(),
        &JsValue::from_f64(params.feed_xy),
    );
    let _ = Reflect::set(
        &toolpath,
        &"travel_feed_xy".into(),
        &JsValue::from_f64(params.travel_feed_xy),
    );
    let _ = Reflect::set(
        &toolpath,
        &"pierce_delay_s".into(),
        &JsValue::from_f64(params.pierce_delay_s),
    );
    let _ = Reflect::set(
        &toolpath,
        &"lead_in_mm".into(),
        &JsValue::from_f64(params.lead_in_mm),
    );
    let _ = Reflect::set(
        &toolpath,
        &"lead_out_mm".into(),
        &JsValue::from_f64(params.lead_out_mm),
    );
    let _ = Reflect::set(
        &toolpath,
        &"kerf_mm".into(),
        &JsValue::from_f64(params.kerf_mm),
    );
    let _ = Reflect::set(
        &toolpath,
        &"torch_on_m3".into(),
        &JsValue::from_str(&params.torch_on_m3),
    );
    let _ = Reflect::set(
        &toolpath,
        &"torch_off_m5".into(),
        &JsValue::from_str(&params.torch_off_m5),
    );
    let _ = Reflect::set(&payload, &"toolpath".into(), &toolpath);

    let machine = Object::new();
    for (id, value) in machine_values {
        let _ = Reflect::set(
            &machine,
            &JsValue::from_str(&id),
            &JsValue::from_str(&value),
        );
    }
    let _ = Reflect::set(&payload, &"machine_settings".into(), &machine);
    let payload = JSON::stringify_with_replacer_and_space(&payload, &JsValue::NULL, &2.into())
        .ok()
        .and_then(|val| val.as_string())
        .unwrap_or_default();

    if let Some(body) = document.body() {
        if let Ok(el) = document.create_element("a") {
            if let Ok(link) = el.dyn_into::<HtmlAnchorElement>() {
                let parts = Array::new();
                parts.push(&JsValue::from_str(&payload));
                if let Ok(blob) = Blob::new_with_str_sequence(&parts) {
                    if let Ok(url) = Url::create_object_url_with_blob(&blob) {
                        link.set_href(&url);
                        link.set_download("toolpath-params.machparams.json");
                        let _ = body.append_child(&link);
                        link.click();
                        let _ = body.remove_child(&link);
                        let _ = Url::revoke_object_url(&url);
                    }
                }
            }
        }
    }
}

pub(crate) fn load_toolpath_params(av: RefAV) {
    let document = av.borrow().document.clone();
    let input = match document.create_element("input") {
        Ok(input) => input,
        Err(_) => return,
    };
    let input: HtmlInputElement = match input.dyn_into() {
        Ok(input) => input,
        Err(_) => return,
    };
    input.set_type("file");
    input.set_accept(".machparams.json,application/json");

    let av_clone = av.clone();
    let input_clone = input.clone();
    let on_change = Closure::wrap(Box::new(move |_event: Event| {
        let files = match input_clone.files() {
            Some(files) => files,
            None => return,
        };
        let file = match files.get(0) {
            Some(file) => file,
            None => return,
        };
        let reader = match FileReader::new() {
            Ok(reader) => reader,
            Err(_) => return,
        };
        let reader_clone = reader.clone();
        let av_inner = av_clone.clone();
        let on_load = Closure::wrap(Box::new(move |_event: Event| {
            let result = reader_clone.result().ok().and_then(|val| val.as_string());
            let Some(json) = result else { return };
            apply_toolpath_params_from_json(av_inner.clone(), json);
        }) as Box<dyn FnMut(_)>);
        reader.set_onload(Some(on_load.as_ref().unchecked_ref()));
        on_load.forget();
        let _ = reader.read_as_text(&file);
    }) as Box<dyn FnMut(_)>);
    input
        .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref())
        .ok();
    on_change.forget();
    input.click();
}

pub(crate) fn apply_toolpath_params_from_json(av: RefAV, json_data: String) {
    let Ok(value) = JSON::parse(&json_data) else {
        return;
    };
    let document = av.borrow().document.clone();
    let mut avb = av.borrow_mut();
    let Some(toolpath_value) = get_prop(&value, "toolpath") else {
        return;
    };
    let params = &mut avb.toolpath_params;

    if let Some(val) = get_prop(&toolpath_value, "feed_xy").and_then(|val| val.as_f64()) {
        params.feed_xy = val;
    }
    if let Some(val) = get_prop(&toolpath_value, "travel_feed_xy").and_then(|val| val.as_f64()) {
        params.travel_feed_xy = val;
    }
    if let Some(val) = get_prop(&toolpath_value, "pierce_delay_s").and_then(|val| val.as_f64()) {
        params.pierce_delay_s = val;
    }
    if let Some(val) = get_prop(&toolpath_value, "lead_in_mm").and_then(|val| val.as_f64()) {
        params.lead_in_mm = val;
    }
    if let Some(val) = get_prop(&toolpath_value, "lead_out_mm").and_then(|val| val.as_f64()) {
        params.lead_out_mm = val;
    }
    if let Some(val) = get_prop(&toolpath_value, "kerf_mm").and_then(|val| val.as_f64()) {
        params.kerf_mm = val;
    }
    if let Some(val) = get_prop(&toolpath_value, "torch_on_m3").and_then(|val| val.as_string()) {
        params.torch_on_m3 = val;
    }
    if let Some(val) = get_prop(&toolpath_value, "torch_off_m5").and_then(|val| val.as_string()) {
        params.torch_off_m5 = val;
    }

    if let Some(machine_val) = get_prop(&value, "machine_settings") {
        let machine_obj = Object::from(machine_val);
        let keys = Object::keys(&machine_obj);
        for key in keys.iter() {
            let Some(id) = key.as_string() else {
                continue;
            };
            let value_str = match Reflect::get(&machine_obj, &key) {
                Ok(val) => val
                    .as_string()
                    .or_else(|| val.as_f64().map(|num| num.to_string()))
                    .unwrap_or_default(),
                Err(_) => String::new(),
            };
            if value_str.is_empty() {
                continue;
            }
            if update_machine_value(&mut avb.machine_groups, &id, &value_str) {
                avb.update_machine_input(&id, &value_str);
            }
        }
    }

    drop(avb);
    let params = av.borrow().toolpath_params.clone();
    let set_value = |id: &str, value: &str| {
        if let Some(el) = document.get_element_by_id(id) {
            if let Ok(input) = el.dyn_into::<HtmlInputElement>() {
                input.set_value(value);
            }
        }
    };
    set_value("tp-feed", &format!("{:.0}", params.feed_xy));
    set_value("tp-travel-feed", &format!("{:.0}", params.travel_feed_xy));
    set_value("tp-pierce", &format!("{:.1}", params.pierce_delay_s));
    set_value("tp-lead-in", &format!("{:.0}", params.lead_in_mm));
    set_value("tp-lead-out", &format!("{:.0}", params.lead_out_mm));
    set_value("tp-kerf", &format!("{:.1}", params.kerf_mm));
    set_value("tp-torch-on", &params.torch_on_m3);
    set_value("tp-torch-off", &params.torch_off_m5);

    update_toolpath_params(av);
}

pub(crate) fn set_callback(
    av: RefAV,
    event: String,
    el: &Element,
    cb: Box<dyn FnMut(RefAV, Event)>,
) -> Result<(), JsValue> {
    let mut cb = cb;
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        cb(av.clone(), event);
    });
    el.add_event_listener_with_callback(&event, closure.as_ref().unchecked_ref())?;
    closure.forget();
    Ok(())
}
