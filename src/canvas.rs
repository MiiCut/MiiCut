// #![cfg(not(test))]
// A macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

use crate::closed_shapes::COperation;
use crate::closed_shapes::ClosedShapesPool;
use crate::dom::*;
use crate::math::*;
use kurbo::{BezPath, PathEl, Point, Vec2};
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{
    CanvasRenderingContext2d, Document, Element, Event, FileList, FileReader, HtmlCanvasElement,
    HtmlElement, HtmlInputElement, KeyboardEvent, MouseEvent, WheelEvent, Window,
};

pub type RefPA = Rc<RefCell<PlayingArea>>;
pub type ElementCallback = Box<dyn Fn(RefPA, Event) + 'static>;

#[derive(Default)]
struct KeysStates {
    crtl_pressed: bool,
    shift_pressed: bool,
}

#[allow(dead_code)]
pub struct PlayingArea {
    csp: ClosedShapesPool,
    //
    pub window: Window,
    pub document: Document,
    pub body: HtmlElement,
    pub canvas: HtmlCanvasElement,
    pub ctx: CanvasRenderingContext2d,

    // DOM
    cm_shape: HtmlElement,
    user_icons: HashMap<&'static str, Option<Element>>,
    tooltip: HtmlElement,
    settings_panel: HtmlElement,
    modal_backdrop: HtmlElement,
    apply_settings_button: HtmlElement,
    settings_width_input: HtmlInputElement,
    settings_height_input: HtmlInputElement,
    draw_styles: DrawStyles,

    he_mouse_worksheet_position: HtmlElement,
    he_viewgrid_element: HtmlElement,
    he_snapgrid_element: HtmlElement,

    // coords: Coords,
    magnet_angle: f64,
    grab_distance: f64,
    draw_cursor: Vec2,

    icon_selected: Icons,
    selection_area: Option<[Vec2; 2]>,
    keys_states: KeysStates,
    mouse: Mouse,

    working_area: Vec2,
    pub global_scale: f64,
    pub canvas_offset: Vec2,
    canvas_offset_ms_dwn: Vec2,
    working_area_visual_grid: f64,
    working_area_snap_grid: f64,
}

///////////////
// Initialization
pub fn create_playing_area(window: Window) -> Result<(), JsValue> {
    log!("Creating playing area");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("should have a body on document");
    let canvas = document
        .get_element_by_id("myCanvas")
        .expect("should have myCanvas on the page")
        .dyn_into::<HtmlCanvasElement>()?;
    let ctx = canvas
        .get_context("2d")?
        .unwrap()
        .dyn_into::<CanvasRenderingContext2d>()?;
    // ctx.scale(1., -1.)?;
    let cm_shape = document
        .get_element_by_id("cm-shape")
        .expect("should have cm-shape id on the page")
        .dyn_into::<HtmlElement>()?;
    let tooltip = document
        .get_element_by_id("tooltip")
        .expect("should have tooltip on the page")
        .dyn_into::<HtmlElement>()?;
    let settings_panel = document
        .get_element_by_id("settingsPanel")
        .expect("should have settingsPanel on the page")
        .dyn_into::<HtmlElement>()?;
    let modal_backdrop = document
        .get_element_by_id("modalBackdrop")
        .expect("should have modalBackdrop on the page")
        .dyn_into::<HtmlElement>()?;
    let apply_settings_button = document
        .get_element_by_id("applyWorksheetSettings")
        .expect("should have applyWorksheetSettings on settingsPanel")
        .dyn_into::<HtmlElement>()?;
    let settings_width_input: HtmlInputElement = document
        .get_element_by_id("worksheetWidthInput")
        .expect("should have settings_width_input on settingsPanel")
        .dyn_into()?;
    let settings_height_input: HtmlInputElement = document
        .get_element_by_id("worksheetHeightInput")
        .expect("should have settings_height_input on settingsPanel")
        .dyn_into()?;
    let mouse_worksheet_position: HtmlElement = document
        .get_element_by_id("status-info-worksheet-pos")
        .expect("should have status-info-worksheet-pos on the page")
        .dyn_into()?;
    let viewgrid_element: HtmlElement = document
        .get_element_by_id("status-viewgrid")
        .expect("should have status-viewgrid on the page")
        .dyn_into()?;
    let snapgrid_element: HtmlElement = document
        .get_element_by_id("status-snapgrid")
        .expect("should have status-snapgrid on the page")
        .dyn_into()?;

    let mut user_icons: HashMap<&'static str, Option<Element>> = HashMap::new();
    use Icons::*;
    user_icons.insert(Arrow.as_str(), None);
    user_icons.insert(Selection.as_str(), None);
    user_icons.insert(Scissors.as_str(), None);
    user_icons.insert(Rectangle.as_str(), None);
    user_icons.insert(RectangleRounded.as_str(), None);
    user_icons.insert(Circle.as_str(), None);
    user_icons.insert(QuarterCircle.as_str(), None);
    user_icons.insert(Oblong.as_str(), None);
    user_icons.insert(QuarterEllipse.as_str(), None);
    user_icons.insert(Line.as_str(), None);
    user_icons.insert(QuadBezier.as_str(), None);
    user_icons.insert(CubicBezier.as_str(), None);

    let document_element = document
        .document_element()
        .ok_or("should have a document element")?;
    let styles = window
        .get_computed_style(&document_element)
        .unwrap()
        .unwrap();

    // Calculation starting parameters
    let (canvas_width, canvas_height) = { (canvas.width() as f64, canvas.height() as f64) };
    // let head_position = WXY { wx: 10., wy: 10. };
    let working_area = Vec2::new(500., 500.);
    settings_width_input.set_value(&working_area.x.to_string());
    settings_height_input.set_value(&working_area.y.to_string());

    let working_area_visual_grid = 10.;
    let working_area_snap_grid = 1.;

    let canvas_offset = Vec2 {
        x: (canvas_width - working_area.x) / 2.,
        y: (canvas_height - working_area.y) / 2.,
    };
    let global_scale = 1.0;

    let playing_area = Rc::new(RefCell::new(PlayingArea {
        csp: ClosedShapesPool::new(),
        window,
        document,
        body,
        canvas,
        ctx,
        //
        cm_shape,
        user_icons,
        tooltip,
        settings_panel,
        modal_backdrop,
        apply_settings_button,
        settings_width_input,
        settings_height_input,
        draw_styles: DrawStyles::build(styles)?,
        he_mouse_worksheet_position: mouse_worksheet_position,
        he_viewgrid_element: viewgrid_element,
        he_snapgrid_element: snapgrid_element,

        magnet_angle: 0.05,
        grab_distance: 20.,
        draw_cursor: Vec2::ZERO,

        // coords: Coords::new(),
        icon_selected: Icons::Arrow,
        selection_area: None,
        keys_states: KeysStates::default(),
        mouse: Mouse::new(),

        // Real word dimensions
        working_area,

        // Zoom
        global_scale,
        canvas_offset,
        canvas_offset_ms_dwn: Vec2::default(),
        working_area_visual_grid,
        working_area_snap_grid,
    }));

    init_window(playing_area.clone())?;
    init_menu(playing_area.clone())?;
    init_canvas(playing_area.clone())?;
    init_context_menu(playing_area.clone())?;
    init_icons(playing_area.clone())?;
    init_settings_panel(playing_area.clone())?;
    init_status(playing_area.clone())?;

    resize_area(playing_area.clone());
    render(playing_area.clone());

    Ok(())
}
fn init_window(pa: RefPA) -> Result<(), JsValue> {
    // Resize event
    let pa_cloned1 = pa.clone();
    let pa_cloned2 = pa.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_window_resize(pa_cloned1.clone(), event);
    });
    pa_cloned2
        .borrow_mut()
        .window
        .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())?;
    closure.forget();

    // Click event
    let pa_cloned1 = pa.clone();
    let pa_cloned2 = pa.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_window_click(pa_cloned1.clone(), event);
    });
    pa_cloned2
        .borrow_mut()
        .window
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
    closure.forget();

    Ok(())
}
fn init_settings_panel(pa: RefPA) -> Result<(), JsValue> {
    let pam = pa.borrow_mut();
    set_callback(
        pa.clone(),
        "click".into(),
        &pam.apply_settings_button,
        Box::new(on_apply_settings_click),
    )?;
    set_callback(
        pa.clone(),
        "click".into(),
        &pam.modal_backdrop,
        Box::new(on_modal_backdrop_click),
    )?;
    Ok(())
}
fn init_icons(pa: RefPA) -> Result<(), JsValue> {
    let mut pam = pa.borrow_mut();
    let document = pam.document.clone();
    for (element_name, element_to_set) in pam.user_icons.iter_mut() {
        if let Some(element) = get_element(&document, element_name).ok() {
            *element_to_set = Some(element);
            set_callback(
                pa.clone(),
                "click".into(),
                &element_to_set.as_ref().unwrap(),
                Box::new(on_icon_click),
            )?;
            set_callback(
                pa.clone(),
                "mouseover".into(),
                &element_to_set.as_ref().unwrap(),
                Box::new(on_icon_mouseover),
            )?;
            set_callback(
                pa.clone(),
                "mouseout".into(),
                &element_to_set.as_ref().unwrap(),
                Box::new(on_icon_mouseout),
            )?;
        }
    }

    Ok(())
}
fn init_context_menu(pa: RefPA) -> Result<(), JsValue> {
    let pam = pa.borrow_mut();
    let document = pam.document.clone();
    //
    let cm_shape_force_horizontal = document
        .get_element_by_id("cm-shape-force-horizontal")
        .unwrap();
    set_callback(
        pa.clone(),
        "click".into(),
        &cm_shape_force_horizontal,
        Box::new(on_cm_shape_force_horizontal),
    )?;
    let cm_shape_force_vertical = document
        .get_element_by_id("cm-shape-force-vertical")
        .unwrap();
    set_callback(
        pa.clone(),
        "click".into(),
        &cm_shape_force_vertical,
        Box::new(on_cm_shape_force_vertical),
    )?;
    let cm_shape_unforce_horizontal = document
        .get_element_by_id("cm-shape-unforce-horizontal")
        .unwrap();
    set_callback(
        pa.clone(),
        "click".into(),
        &cm_shape_unforce_horizontal,
        Box::new(on_cm_shape_unforce_horizontal),
    )?;
    let cm_shape_unforce_vertical = document
        .get_element_by_id("cm-shape-unforce-vertical")
        .unwrap();
    set_callback(
        pa.clone(),
        "click".into(),
        &cm_shape_unforce_vertical,
        Box::new(on_cm_shape_unforce_vertical),
    )?;
    Ok(())
}
fn init_canvas(pa: RefPA) -> Result<(), JsValue> {
    let mut element = &pa.borrow_mut().canvas;
    set_callback(
        pa.clone(),
        "mousedown".into(),
        element,
        Box::new(on_mouse_down),
    )?;
    set_callback(
        pa.clone(),
        "contextmenu".into(),
        element,
        Box::new(on_context_menu),
    )?;
    set_callback(
        pa.clone(),
        "mousemove".into(),
        &mut element,
        Box::new(on_mouse_move),
    )?;
    set_callback(
        pa.clone(),
        "mouseup".into(),
        &mut element,
        Box::new(on_mouse_up),
    )?;
    set_callback(
        pa.clone(),
        "mouseenter".into(),
        &mut element,
        Box::new(on_mouse_enter),
    )?;
    set_callback(
        pa.clone(),
        "mouseleave".into(),
        &mut element,
        Box::new(on_mouse_leave),
    )?;
    set_callback(
        pa.clone(),
        "wheel".into(),
        &mut element,
        Box::new(on_mouse_wheel),
    )?;
    set_callback(
        pa.clone(),
        "keydown".into(),
        &mut element,
        Box::new(on_keydown),
    )?;
    set_callback(pa.clone(), "keyup".into(), &mut element, Box::new(on_keyup))?;
    Ok(())
}
fn init_menu(pa: RefPA) -> Result<(), JsValue> {
    let pam = pa.borrow_mut();
    let document = pam.document.clone();

    let load_element = document.get_element_by_id("load-option").unwrap();
    let load_element: HtmlElement = load_element.dyn_into::<HtmlElement>()?;

    let save_element = document.get_element_by_id("save-option").unwrap();
    let save_element: HtmlElement = save_element.dyn_into::<HtmlElement>()?;

    let file_input = document.get_element_by_id("file-input").unwrap();
    let file_input: HtmlElement = file_input.dyn_into::<HtmlElement>()?;

    let file_input_clone = file_input.clone();
    let on_load = Closure::wrap(Box::new(move || {
        // Trigger a click event on the file input element to open the file dialog
        file_input_clone.click();
    }) as Box<dyn FnMut()>);

    let on_save = Closure::wrap(Box::new(move || {
        // Your save action here
    }) as Box<dyn FnMut()>);

    load_element.add_event_listener_with_callback("click", on_load.as_ref().unchecked_ref())?;
    on_load.forget(); // Leaks memory, but we need to do this to keep the callback alive

    save_element.add_event_listener_with_callback("click", on_save.as_ref().unchecked_ref())?;
    on_save.forget(); // Leaks memory, but we need to do this to keep the callback alive

    drop(pam);
    // Set up an event listener to handle file selection
    let on_file_select = Closure::wrap(Box::new(move || {
        let pa_clone = pa.clone();
        // Get the files from the file input element
        if let Some(file_input) = document.get_element_by_id("file-input") {
            let file_input: HtmlInputElement = file_input.dyn_into().unwrap();
            let files = js_sys::Reflect::get(&file_input.into(), &"files".into())
                .unwrap()
                .dyn_into::<FileList>()
                .unwrap();
            if let Some(file) = files.get(0) {
                // let file_name = file.name();
                let file_reader = FileReader::new().unwrap();

                let on_load = Closure::wrap(Box::new(move |event: Event| {
                    let target = event.target().unwrap();
                    let file_reader: FileReader = target.dyn_into().unwrap();
                    let result = file_reader.result().unwrap();
                    if let Some(content) = result.as_string() {
                        convert_svg_to_shapes(pa_clone.clone(), content);
                        drop(pa_clone.borrow_mut());
                        render(pa_clone.clone());
                    }
                }) as Box<dyn FnMut(_)>);

                file_reader
                    .add_event_listener_with_callback("load", on_load.as_ref().unchecked_ref())
                    .unwrap();
                on_load.forget(); // Avoid memory leak

                file_reader.read_as_text(&file).unwrap();
            }
        }
    }) as Box<dyn FnMut()>);

    file_input
        .add_event_listener_with_callback("change", on_file_select.as_ref().unchecked_ref())?;
    on_file_select.forget(); // Leaks memory, but we need to do this to keep the callback alive

    Ok(())
}
fn init_status(pa: RefPA) -> Result<(), JsValue> {
    let pam = pa.borrow_mut();
    let _document = pam.document.clone();

    Ok(())
}
fn set_callback(
    pa: RefPA,
    event_str: String,
    element: &Element,
    callback: ElementCallback,
) -> Result<(), JsValue> {
    let event_str_cloned = event_str.clone();
    let callback = Box::new(move |pa: RefPA, e: Event| {
        if let Ok(mouse_event) = e.clone().dyn_into::<MouseEvent>() {
            if mouse_event.type_().as_str() == event_str_cloned {
                callback(pa.clone(), e);
            }
        } else {
            if let Ok(keyboard_event) = e.clone().dyn_into::<KeyboardEvent>() {
                if keyboard_event.type_().as_str() == event_str_cloned {
                    callback(pa.clone(), e);
                }
            }
        }
    });
    let closure = Closure::wrap(Box::new(move |event: Event| {
        callback(pa.clone(), event);
    }) as Box<dyn FnMut(Event)>);
    element
        .add_event_listener_with_callback(&event_str, closure.as_ref().unchecked_ref())
        .map_err(|e| JsValue::from_str(&format!("Failed to add event listener: {:?}", e)))?;
    closure.forget();

    Ok(())
}
fn convert_svg_to_shapes(_pa: RefPA, _svg_data: String) {
    // let mut pam = pa.borrow_mut();
    // let grp_id = pam.data_pools.create_group_id();
    // for event in svg::parser::Parser::new(&svg_data).into_iter() {
    //     match event {
    //         svg::parser::Event::Tag(svg::node::element::tag::Path, _, attributes) => {
    //             let data = attributes.get("d").unwrap();
    //             let data = svg::node::element::path::Data::parse(data).unwrap();
    //             let mut current_position = Point::default();
    //             let mut start_position = Point::default();
    //             let mut last_quad_control_point: Option<Point> = None;
    //             let mut last_cubic_control_point: Option<Point> = None;
    //             for command in data.iter() {
    //                 let command_clone = command.clone();
    //                 use svg::node::element::path::*;
    //                 match command_clone {
    //                     Command::Move(postype, params) => {
    //                         if params.len() == 2 {
    //                             current_position = match postype {
    //                                 Position::Absolute => Point {
    //                                     x: params[0] as f64,
    //                                     y: params[1] as f64,
    //                                 },
    //                                 Position::Relative => Point {
    //                                     x: params[0] as f64 + current_position.x,
    //                                     y: params[1] as f64 + current_position.y,
    //                                 },
    //                             };
    //                             start_position = current_position;
    //                             last_quad_control_point = None;
    //                             last_cubic_control_point = None;
    //                         }
    //                     }
    //                     _ => (), // Command::Line(postype, params) => {
    //                              //     if params.len() % 2 == 0 {
    //                              //         let nb_curves = params.len() / 2;
    //                              //         for curve in 0..nb_curves {
    //                              //             let end_point = Point {
    //                              //                 wx: params[2 * curve] as f64,
    //                              //                 wy: params[2 * curve + 1] as f64,
    //                              //             };
    //                              //             let new_position = match postype {
    //                              //                 Position::Absolute => end_point,
    //                              //                 Position::Relative => current_position + end_point,
    //                              //             };
    //                              //             if let Some(shape) = Line::new(&current_position, &new_position)
    //                              //             {
    //                              //                 let sh_id = pam.data_pools.insert_shape(Box::new(shape));
    //                              //                 pam.data_pools.set_shape_selected(&sh_id, true);
    //                              //                 pam.data_pools.set_shape_group(&grp_id, &sh_id);
    //                              //             }
    //                              //             current_position = new_position;
    //                              //             last_quad_control_point = None;
    //                              //             last_cubic_control_point = None;
    //                              //         }
    //                              //     }
    //                              // }
    //                              // Command::HorizontalLine(postype, params) => {
    //                              //     for curve in 0..params.len() {
    //                              //         let end_point = Point {
    //                              //             wx: params[curve] as f64,
    //                              //             wy: current_position.y,
    //                              //         };
    //                              //         let new_position = match postype {
    //                              //             Position::Absolute => end_point,
    //                              //             Position::Relative => current_position + end_point,
    //                              //         };
    //                              //         if let Some(shape) = Line::new(&current_position, &new_position) {
    //                              //             let sh_id = pam.data_pools.insert_shape(Box::new(shape));
    //                              //             pam.data_pools.set_shape_selected(&sh_id, true);
    //                              //             pam.data_pools.set_shape_group(&grp_id, &sh_id);
    //                              //         }
    //                              //         current_position = new_position;
    //                              //         last_quad_control_point = None;
    //                              //         last_cubic_control_point = None;
    //                              //     }
    //                              // }
    //                              // Command::VerticalLine(postype, params) => {
    //                              //     for curve in 0..params.len() {
    //                              //         let end_point = Point {
    //                              //             wx: current_position.x,
    //                              //             wy: params[curve] as f64,
    //                              //         };
    //                              //         let new_position = match postype {
    //                              //             Position::Absolute => end_point,
    //                              //             Position::Relative => current_position + end_point,
    //                              //         };
    //                              //         if let Some(shape) = Line::new(&current_position, &new_position) {
    //                              //             let sh_id = pam.data_pools.insert_shape(Box::new(shape));
    //                              //             pam.data_pools.set_shape_selected(&sh_id, true);
    //                              //             pam.data_pools.set_shape_group(&grp_id, &sh_id);
    //                              //         }
    //                              //         current_position = new_position;
    //                              //         last_quad_control_point = None;
    //                              //         last_cubic_control_point = None;
    //                              //     }
    //                              // }
    //                              // Command::QuadraticCurve(postype, params) => {
    //                              //     if params.len() % 4 == 0 {
    //                              //         let nb_curves = params.len() / 4;
    //                              //         for curve in 0..nb_curves {
    //                              //             let mut control_point = Point {
    //                              //                 wx: params[4 * curve] as f64,
    //                              //                 wy: params[4 * curve + 1] as f64,
    //                              //             };
    //                              //             let end_point = Point {
    //                              //                 wx: params[4 * curve + 2] as f64,
    //                              //                 wy: params[4 * curve + 3] as f64,
    //                              //             };
    //                              //             let new_position = match postype {
    //                              //                 Position::Absolute => end_point,
    //                              //                 Position::Relative => {
    //                              //                     control_point += current_position;
    //                              //                     current_position + end_point
    //                              //                 }
    //                              //             };
    //                              //             if let Some(shape) = QuadBezier::new(
    //                              //                 &current_position,
    //                              //                 &control_point,
    //                              //                 &new_position,
    //                              //             ) {
    //                              //                 let sh_id = pam.data_pools.insert_shape(Box::new(shape));
    //                              //                 pam.data_pools.set_shape_selected(&sh_id, true);
    //                              //                 pam.data_pools.set_shape_group(&grp_id, &sh_id);
    //                              //             }
    //                              //             current_position = new_position;
    //                              //             last_quad_control_point = Some(control_point);
    //                              //             last_cubic_control_point = None;
    //                              //         }
    //                              //     }
    //                              // }
    //                              // Command::SmoothQuadraticCurve(postype, params) => {
    //                              //     if params.len() % 2 == 0 {
    //                              //         let nb_curves = params.len() / 2;
    //                              //         for curve in 0..nb_curves {
    //                              //             let control_point =
    //                              //                 if let Some(last_ctrl_pt) = last_quad_control_point {
    //                              //                     current_position + (current_position - last_ctrl_pt)
    //                              //                 } else {
    //                              //                     current_position
    //                              //                 };
    //                              //             let end_point = Point {
    //                              //                 wx: params[2 * curve] as f64,
    //                              //                 wy: params[2 * curve + 1] as f64,
    //                              //             };
    //                              //             let new_position = match postype {
    //                              //                 Position::Absolute => end_point,
    //                              //                 Position::Relative => current_position + end_point,
    //                              //             };
    //                              //             if let Some(shape) = QuadBezier::new(
    //                              //                 &current_position,
    //                              //                 &control_point,
    //                              //                 &new_position,
    //                              //             ) {
    //                              //                 let sh_id = pam.data_pools.insert_shape(Box::new(shape));
    //                              //                 pam.data_pools.set_shape_selected(&sh_id, true);
    //                              //                 pam.data_pools.set_shape_group(&grp_id, &sh_id);
    //                              //             }
    //                              //             current_position = new_position;
    //                              //             last_quad_control_point = Some(control_point);
    //                              //             last_cubic_control_point = None;
    //                              //         }
    //                              //     }
    //                              // }
    //                              // Command::CubicCurve(postype, params) => {
    //                              //     if params.len() % 6 == 0 {
    //                              //         let nb_curves = params.len() / 6;
    //                              //         for curve in 0..nb_curves {
    //                              //             let mut control_point1 = Point {
    //                              //                 wx: params[6 * curve] as f64,
    //                              //                 wy: params[6 * curve + 1] as f64,
    //                              //             };
    //                              //             let mut control_point2 = Point {
    //                              //                 wx: params[6 * curve + 2] as f64,
    //                              //                 wy: params[6 * curve + 3] as f64,
    //                              //             };
    //                              //             let end_point = Point {
    //                              //                 wx: params[6 * curve + 4] as f64,
    //                              //                 wy: params[6 * curve + 5] as f64,
    //                              //             };
    //                              //             let new_position = match postype {
    //                              //                 Position::Absolute => end_point,
    //                              //                 Position::Relative => {
    //                              //                     control_point1 += current_position;
    //                              //                     control_point2 += current_position;
    //                              //                     current_position + end_point
    //                              //                 }
    //                              //             };
    //                              //             if let Some(shape) = CubicBezier::new(
    //                              //                 &current_position,
    //                              //                 &control_point1,
    //                              //                 &control_point2,
    //                              //                 &new_position,
    //                              //             ) {
    //                              //                 let sh_id = pam.data_pools.insert_shape(Box::new(shape));
    //                              //                 pam.data_pools.set_shape_selected(&sh_id, true);
    //                              //                 pam.data_pools.set_shape_selected(&sh_id, true);
    //                              //                 pam.data_pools.set_shape_group(&grp_id, &sh_id);
    //                              //             }
    //                              //             current_position = new_position;
    //                              //             last_quad_control_point = None;
    //                              //             last_cubic_control_point = Some(control_point2);
    //                              //         }
    //                              //     }
    //                              // }
    //                              // Command::SmoothCubicCurve(postype, params) => {
    //                              //     if params.len() % 4 == 0 {
    //                              //         let nb_curves = params.len() / 4;
    //                              //         for curve in 0..nb_curves {
    //                              //             let control_point1 =
    //                              //                 if let Some(last_ctrl_pt) = last_cubic_control_point {
    //                              //                     current_position + (current_position - last_ctrl_pt)
    //                              //                 } else {
    //                              //                     current_position
    //                              //                 };
    //                              //             let mut control_point2 = Point {
    //                              //                 wx: params[4 * curve] as f64,
    //                              //                 wy: params[4 * curve + 1] as f64,
    //                              //             };
    //                              //             let end_point = Point {
    //                              //                 wx: params[4 * curve + 2] as f64,
    //                              //                 wy: params[4 * curve + 3] as f64,
    //                              //             };
    //                              //             let new_position = match postype {
    //                              //                 Position::Absolute => end_point,
    //                              //                 Position::Relative => {
    //                              //                     control_point2 += current_position;
    //                              //                     current_position + end_point
    //                              //                 }
    //                              //             };
    //                              //             if let Some(shape) = CubicBezier::new(
    //                              //                 &current_position,
    //                              //                 &control_point1,
    //                              //                 &control_point2,
    //                              //                 &new_position,
    //                              //             ) {
    //                              //                 let sh_id = pam.data_pools.insert_shape(Box::new(shape));
    //                              //                 pam.data_pools.set_shape_selected(&sh_id, true);
    //                              //                 pam.data_pools.set_shape_selected(&sh_id, true);
    //                              //                 pam.data_pools.set_shape_group(&grp_id, &sh_id);
    //                              //             }
    //                              //             current_position = new_position;
    //                              //             last_quad_control_point = None;
    //                              //             last_cubic_control_point = Some(control_point2);
    //                              //         }
    //                              //     }
    //                              // }
    //                              // Command::EllipticalArc(_postype, _params) => {}
    //                              // Command::Close => {
    //                              //     if let Some(shape) = Line::new(&current_position, &start_position) {
    //                              //         let sh_id = pam.data_pools.insert_shape(Box::new(shape));
    //                              //         pam.data_pools.set_shape_selected(&sh_id, true);
    //                              //         pam.data_pools.set_shape_group(&grp_id, &sh_id);
    //                              //     }
    //                              //     current_position = start_position;
    //                              //     last_quad_control_point = None;
    //                              //     last_cubic_control_point = None;
    //                              // }
    //                 }
    //             }
    //         }
    //         _ => {}
    //     }
    // }
}

fn update(pam: &mut RefMut<'_, PlayingArea>) -> Result<(), MyError> {
    let grab_precision = pam.grab_distance / pam.global_scale;
    let _magnet_angle = pam.magnet_angle;

    match pam.icon_selected.clone() {
        Icons::Arrow => match pam.mouse.get_mouse_state() {
            MouseState::LeftDown(cursor_pos) => {
                pam.csp.set_selection(cursor_pos, grab_precision);
                pam.csp.save_positions();
                Ok(())
            }
            MouseState::LeftDownMove(pos_dwn, cursor_pos) => {
                pam.csp.highlight_object(cursor_pos, grab_precision);
                pam.csp.move_selection(pos_dwn, cursor_pos)?;
                Ok(())
            }
            MouseState::LeftUp(_cursor_pos) => {
                // match pam.dp.get_selection() {
                //     DrawObject::Vertex(vid_sel_old) => {
                //         // pam.dp
                //         //     .set_selection(&cursor_pos, grab_precision, Some(vid_sel_old))?;
                //         // match pam.dp.get_selection() {
                //         //     DrawObject::Vertex(vid_sel_new) => {
                //         //         pam.dp.merge_vertices(&vid_sel_old, &vid_sel_new)?
                //         //     }
                //         //     _ => pam.dp.force_selection_to_vertex(&vid_sel_old),
                //         // }
                //     }
                //     _ => (),
                // };
                Ok(())
            }
            MouseState::LeftUpMove(_, cursor_pos) => {
                pam.csp.highlight_object(cursor_pos, grab_precision);
                Ok(())
            }
            MouseState::RightDown(_) => Ok(pam.canvas_offset_ms_dwn = pam.canvas_offset),
            MouseState::RightDownMove(_, _) => Ok(pam.canvas_offset =
                (pam.mouse.get_canvas_mouse_pos() - pam.mouse.get_canvas_mouse_pos_ms_dwn())
                    + pam.canvas_offset_ms_dwn),
            _ => Ok(()),
        },
        Icons::Selection => Ok(()),
        Icons::Scissors => Ok(()),
        Icons::Rectangle | Icons::RectangleRounded | Icons::Circle | Icons::Oblong => {
            // Supposons un rectangle
            match pam.mouse.get_mouse_state() {
                MouseState::LeftDown(cursor_pos) => {
                    if let Some(cshid) = pam.csp.on_creation {
                        // We were drawing a new shape, finish all
                        pam.csp.clear_selection_all(cshid)?;
                        pam.csp.on_creation = None;
                        go_to_arrow_tool(pam);
                        Ok(())
                    } else {
                        match pam.icon_selected.clone() {
                            Icons::Rectangle => {
                                pam.csp.on_creation =
                                    Some(pam.csp.create_bundle_rectangle(cursor_pos))
                            }

                            Icons::RectangleRounded => {
                                pam.csp.on_creation =
                                    Some(pam.csp.create_bundle_rectangle_rounded(cursor_pos))
                            }
                            Icons::Circle => {
                                pam.csp.on_creation = Some(pam.csp.create_bundle_circle(cursor_pos))
                            }
                            Icons::Oblong => {
                                pam.csp.on_creation = Some(pam.csp.create_bundle_oblong(cursor_pos))
                            }
                            _ => (),
                        }

                        Ok(())
                    }
                }
                MouseState::LeftDownMove(pos_dwn, cursor_pos)
                | MouseState::LeftUpMove(pos_dwn, cursor_pos) => {
                    pam.draw_cursor = cursor_pos;
                    pam.csp.move_selection(pos_dwn, cursor_pos)?;
                    Ok(())
                }
                MouseState::RightDown(_) => {
                    if let Some(cshid) = pam.csp.on_creation {
                        pam.csp.delete_shape(cshid)?;
                        pam.csp.on_creation = None;
                        go_to_arrow_tool(pam);
                    }
                    Ok(())
                }
                _ => Ok(()),
            }
        }
        Icons::Line
        | Icons::QuadBezier
        | Icons::CubicBezier
        | Icons::QuarterCircle
        | Icons::QuarterEllipse => {
            // let shape_kind = ShapeKind::Custom(ShapeCustom);
            match pam.mouse.get_mouse_state() {
                // MouseState::LeftDown(cursor_pos) => match pam.dp.get_selection() {
                //     DrawObject::None | DrawObject::Shape(_) => {
                //         // pam.dp.set_selection(&cursor_pos, grab_precision, None)?;
                //         // Ok(pam
                //         //     .dp
                //         //     .create_bundle(&cursor_pos, shape_kind, Constraint::F)?)
                //         Ok(())
                //     }
                //     DrawObject::Vertex(vid_sel_old) => {
                //         // pam.dp
                //         //     .set_selection(&cursor_pos, grab_precision, Some(vid_sel_old))?;
                //         // match pam.dp.get_selection() {
                //         //     DrawObject::Vertex(vid_sel_new)
                //         //         if pam.dp.vertices.get(&vid_sel_new)?.can_add_shape() =>
                //         //     {
                //         //         pam.dp.merge_vertices(&vid_sel_old, &vid_sel_new)?;
                //         //         pam.dp.clear_selection();
                //         //         go_to_arrow_tool(pam);
                //         //     }
                //         //     _ => {
                //         //         pam.dp.force_selection_to_vertex(&vid_sel_old);
                //         //         pam.dp
                //         //             .create_bundle(&cursor_pos, shape_kind, Constraint::F)?;
                //         //     }
                //         // }
                //         Ok(())
                //     }
                // },
                // MouseState::LeftDownMove(pos_dwn, cursor_pos)
                // | MouseState::LeftUpMove(pos_dwn, cursor_pos) => {
                //     pam.draw_cursor = cursor_pos;
                //     pam.dp.highlight_object(cursor_pos, grab_precision);

                //     // if let DrawObject::Vertex(v_sel) = pam.dp.get_selection() {
                //     //     pam.dp.move_new_vertex(&v_sel, &cursor_pos)?;
                //     // }
                //     pam.dp.move_object_selected(pos_dwn, cursor_pos)?;
                //     Ok(())
                // }
                // MouseState::RightDown(_) => {
                //     if let DrawObject::Vertex(vid_sel) = pam.dp.get_selection() {
                //         pam.dp.delete_vertex(vid_sel)?;
                //     }
                //     Ok(())
                // }
                _ => Ok(()),
            }
        }
    }
}

fn on_mouse_move(pa: RefPA, event: Event) {
    let mut pam = pa.borrow_mut();
    let rect = pam.canvas.get_bounding_client_rect();
    let scale = pam.global_scale;
    let canvas_offset = pam.canvas_offset;

    pam.mouse
        .update_mouse(&rect, scale, &canvas_offset, &event, SystemMouse::Move);

    if let Some(e) = update(&mut pam).err() {
        log!("ERROR: {}", e);
    }
    update_status_bar(&mut pam);
    drop(pam);
    render(pa.clone());
}
fn on_mouse_down(pa: RefPA, event: Event) {
    let mut pam = pa.borrow_mut();
    let rect = pam.canvas.get_bounding_client_rect();
    let scale = pam.global_scale;
    let canvas_offset = pam.canvas_offset;
    pam.mouse
        .update_mouse(&rect, scale, &canvas_offset, &event, SystemMouse::Down);

    // Hide the context menu when clicking elsewhere
    hide_cm_shape(&mut pam);

    if let Some(e) = update(&mut pam).err() {
        log!("ERROR: {}", e);
    }
    update_status_bar(&mut pam);
    drop(pam);
    render(pa.clone());
}
fn on_mouse_up(pa: RefPA, event: Event) {
    let mut pam = pa.borrow_mut();
    let rect = pam.canvas.get_bounding_client_rect();
    let scale = pam.global_scale;
    let canvas_offset = pam.canvas_offset;

    pam.mouse
        .update_mouse(&rect, scale, &canvas_offset, &event, SystemMouse::Up);

    if let Some(e) = update(&mut pam).err() {
        log!("ERROR: {}", e);
    }
    update_status_bar(&mut pam);
    drop(pam);
    render(pa.clone());
}

fn update_status_bar(_pam: &mut RefMut<'_, PlayingArea>) {
    // Display: update mouse world position
    // let cp = pam.draw_cursor;

    // pam.he_mouse_worksheet_position
    //     .set_text_content(Some(&format!("( {:.3} , {:.3} )", cp.x, cp.y)));
    // //
    // let bold_status = match pam.data.get_bold() {
    //     Bolded::BoldVertex(vid_bold) => {
    //         if let Some(vid_link) = vid_bold.get_link() {
    //             &format!(
    //                 "BOLD V{}{}, VLINK:{}{}",
    //                 vid_bold.get_dv().get_id(),
    //                 vid_bold.get_type(),
    //                 vid_link.get_dv(),
    //                 vid_link.get_type()
    //             )
    //         } else {
    //             &format!(
    //                 "BOLD V{}{}",
    //                 vid_bold.get_dv().get_id(),
    //                 vid_bold.get_type()
    //             )
    //         }
    //     }
    //     Bolded::BoldShape(sh_id) => &format!("HIGH SH: {:?}", sh_id),
    //     Bolded::BoldNone => &format!("HIGH None"),
    // };

    // pam.he_viewgrid_element.set_text_content(Some(bold_status));

    // let sel_status = match pam.data.get_selected() {
    //     Selected::SelVertex(vid) => &format!("SEL V{}{}", vid.get_dv(), vid.get_type()),
    //     Selected::SelShape(sh_id) => &format!("SEL SH:{}", *sh_id),
    //     Selected::SelNone => &format!("SEL None"),
    // };
    // pam.he_snapgrid_element.set_text_content(Some(sel_status));
}
fn on_mouse_wheel(pa: RefPA, event: Event) {
    if let Ok(wheel_event) = event.dyn_into::<WheelEvent>() {
        wheel_event.prevent_default();
        let mut pam = pa.borrow_mut();
        let zoom_factor = 0.05;

        let old_scale = pam.global_scale;

        // Get mouse position relative to the canvas
        let rect = pam.canvas.get_bounding_client_rect();
        let canvas_mouse_pos = Point {
            x: wheel_event.client_x() as f64 - rect.left(),
            y: wheel_event.client_y() as f64 - rect.top(),
        };

        // Determine the new scale
        let new_scale = if wheel_event.delta_y() < 0. {
            // Zoom in
            (old_scale * (1.0 + zoom_factor)).min(10.0)
        } else {
            // Zoom out
            (old_scale / (1.0 + zoom_factor)).max(0.2)
        };

        let new_canvas_offset_x = pam.canvas_offset.x
            - (new_scale - old_scale) * (canvas_mouse_pos.x - pam.canvas_offset.x) / old_scale;
        let new_canvas_offset_y = pam.canvas_offset.y
            - (new_scale - old_scale) * (canvas_mouse_pos.y - pam.canvas_offset.y) / old_scale;

        pam.canvas_offset = Vec2 {
            x: new_canvas_offset_x,
            y: new_canvas_offset_y,
        };
        pam.global_scale = new_scale;
        drop(pam);
        render(pa);
    }
}
fn on_mouse_enter(pa: RefPA, _event: Event) {
    let mut _pam = pa.borrow_mut();
    // pam.mouse_state = MouseState::NoButton;
}
fn on_mouse_leave(pa: RefPA, _event: Event) {
    let mut _pam = pa.borrow_mut();
    // pam.mouse_state = MouseState::NoButton;
}
fn on_keydown(pa: RefPA, event: Event) {
    event.prevent_default();
    if let Ok(keyboard_event) = event.dyn_into::<KeyboardEvent>() {
        let mut pam = pa.borrow_mut();
        if keyboard_event.key() == " " {
            // match pam.dp.get_selection() {
            //     // DrawObject::Shape(shid) => {
            //     //     if let Some(shape) = pam.dp.shapes.get_mut(&shid).ok() {
            //     //         shape.toggle_prop();
            //     //     }
            //     // }
            //     _ => (),
            // }
        }
        if keyboard_event.key() == "d" {
            log!("creation: {:?}", pam.csp.on_creation);
        }
        if keyboard_event.key() == "Delete" || keyboard_event.key() == "Backspace" {
            if let Err(e) = pam.csp.delete_object_selected() {
                log!("Error deleting object: {}", e);
            }
        }
        if keyboard_event.key() == "Escape" {
            // if let DrawObject::Vertex(vid) = pam.dp.get_selection() {
            //     if let Err(e) = pam.dp.delete_vertex(vid) {
            //         log!("Error deleting vertex: {}", e);
            //     }
            // }
            pam.csp.on_creation = None;
            go_to_arrow_tool(&mut pam);
        }
        if keyboard_event.key() == "Control" || keyboard_event.key() == "Meta" {
            pam.keys_states.crtl_pressed = true;
        }
        if keyboard_event.key() == "Shift" {
            pam.keys_states.shift_pressed = true;
        }
        if keyboard_event.key() == "Tab" {
            // let icon_selected_new = match pam.icon_selected {
            //     Icons::Line => Icons::QuadBezier,
            //     Icons::QuadBezier => Icons::CubicBezier,
            //     Icons::CubicBezier => Icons::QuarterCircle,
            //     Icons::QuarterCircle => Icons::QuarterEllipse,
            //     Icons::QuarterEllipse => Icons::Line,
            //     _ => pam.icon_selected,
            // };
            // if let DrawObject::Vertex(v_sel) = pam.dp.get_selection() {
            //     if let Some(vertex) = pam.dp.vertices.get(&v_sel).ok() {
            //         if let Some(shid) = vertex.get_shid().ok() {
            //             if let Some(shape) = pam.dp.shapes.get(&shid).ok() {
            //                 //
            //             }
            //         }
            //     }
            // }

            // if pam.icon_selected != icon_selected_new {
            //     html_deselect_icons(&pam);
            //     html_select_icon(&pam, pam.icon_selected.as_str());
            // }
        }
        // if keyboard_event.key() == "c" {
        //     if pam.ctrl_or_meta_pressed {
        //         let copy = pam
        //             .shapes
        //             .iter()
        //             .filter(|shape| shape.get_handle_selected() < -1)
        //             .cloned()
        //             .collect();
        //         pam.shape_buffer_copy_paste = copy;
        //     }
        // }
        drop(pam);
        render(pa.clone());
    }
}
fn on_keyup(pa: RefPA, event: Event) {
    if let Ok(keyboard_event) = event.dyn_into::<KeyboardEvent>() {
        let mut pam = pa.borrow_mut();
        if keyboard_event.key() == "Control" || keyboard_event.key() == "Meta" {
            pam.keys_states.crtl_pressed = false;
        }
        if keyboard_event.key() == "Shift" {
            pam.keys_states.shift_pressed = false;
        }
    }
}
fn on_context_menu(pa: RefPA, event: Event) {
    // Prevent the default context menu from appearing
    event.prevent_default();
    let mut pam = pa.borrow_mut();
    show_context_menu(&mut pam);
}
fn show_context_menu(pam: &mut RefMut<'_, PlayingArea>) {
    match pam.icon_selected {
        Icons::Line
        | Icons::QuadBezier
        | Icons::CubicBezier
        | Icons::QuarterCircle
        | Icons::QuarterEllipse => {
            // User simply want to end the drawing mode, no context menu
            pam.csp.on_creation = None;
            go_to_arrow_tool(pam);
        }
        _ => match pam.csp.on_creation {
            None => (),
            Some(_) => {
                // if let Some(horizontal) = pam.dp.is_horizontal(&shid).ok() {
                //     if horizontal {
                //         if let Some(shape) = pam.dp.shapes.get(&shid).ok() {
                //             match shape.get_cstr() {
                //                 Constraint::F => {
                //                     show_cm_shape(pam, "cm-shape-force-horizontal");
                //                     return;
                //                 }
                //                 Constraint::H => {
                //                     show_cm_shape(pam, "cm-shape-unforce-horizontal");
                //                     return;
                //                 }
                //                 Constraint::V => return,
                //             }
                //         }
                //     }
                // }
                // if let Some(vertical) = pam.dp.is_vertical(&shid).ok() {
                //     if vertical {
                //         if let Some(shape) = pam.dp.shapes.get(&shid).ok() {
                //             match shape.get_cstr() {
                //                 Constraint::F => {
                //                     show_cm_shape(pam, "cm-shape-force-vertical");
                //                 }
                //                 Constraint::V => {
                //                     show_cm_shape(pam, "cm-shape-unforce-vertical");
                //                 }
                //                 Constraint::H => return,
                //             }
                //         }
                //     }
                // }
            }
        },
    }
}
fn _show_cm_shape(pam: &mut RefMut<'_, PlayingArea>, action_to_show: &str) {
    // Display the context menu container
    pam.cm_shape
        .style()
        .set_property("display", "block")
        .unwrap();

    // Position the context menu
    let mouse_client = pam.mouse.get_mouse_client();
    pam.cm_shape
        .style()
        .set_property("top", &format!("{}px", mouse_client.y))
        .unwrap();
    pam.cm_shape
        .style()
        .set_property("left", &format!("{}px", mouse_client.x))
        .unwrap();

    // List of all action IDs
    let actions = [
        "cm-shape-force-horizontal",
        "cm-shape-force-vertical",
        "cm-shape-unforce-horizontal",
        "cm-shape-unforce-vertical",
    ];

    // Show only the specified action, hide others
    for action in actions.iter() {
        if let Some(element) = pam
            .cm_shape
            .query_selector(&format!("#{}", action))
            .ok()
            .flatten()
        {
            if let Some(html_element) = element.dyn_ref::<HtmlElement>() {
                if action == &action_to_show {
                    html_element
                        .style()
                        .set_property("display", "inline")
                        .unwrap();
                } else {
                    html_element
                        .style()
                        .set_property("display", "none")
                        .unwrap();
                }
            } else {
                log!("Failed to cast action {} to HtmlElement", action);
            }
        }
    }
}
fn hide_cm_shape(pam: &mut RefMut<'_, PlayingArea>) {
    pam.cm_shape
        .style()
        .set_property("display", "none")
        .unwrap();
}
fn on_cm_shape_force_horizontal(pa: RefPA, _event: Event) {
    let mut pam = pa.borrow_mut();

    // if let DrawObject::Shape(shid) = pam.dp.get_selection() {
    //     log!("shape before: {:?}", shid);
    //     // if let Some(shape) = pam.dp.shapes.get_mut(&shid).ok() {
    //     //     shape.set_cstr(Constraint::H);
    //     // }
    // }
    hide_cm_shape(&mut pam);
}
fn on_cm_shape_force_vertical(pa: RefPA, _event: Event) {
    let mut pam = pa.borrow_mut();
    // if let DrawObject::Shape(shid) = pam.dp.get_selection() {
    //     log!("shape before: {:?}", shid);
    //     // if let Some(shape) = pam.dp.shapes.get_mut(&shid).ok() {
    //     //     shape.set_cstr(Constraint::V);
    //     // }
    // }
    hide_cm_shape(&mut pam);
}
fn on_cm_shape_unforce_horizontal(pa: RefPA, _event: Event) {
    let mut pam = pa.borrow_mut();
    // if let DrawObject::Shape(shid) = pam.dp.get_selection() {
    //     log!("shape before: {:?}", shid);
    //     // if let Some(shape) = pam.dp.shapes.get_mut(&shid).ok() {
    //     //     shape.set_cstr(Constraint::F);
    //     // }
    // }
    hide_cm_shape(&mut pam);
}
fn on_cm_shape_unforce_vertical(pa: RefPA, _event: Event) {
    let mut pam = pa.borrow_mut();
    // if let DrawObject::Shape(shid) = pam.dp.get_selection() {
    //     log!("shape before: {:?}", shid);
    //     // if let Some(shape) = pam.dp.shapes.get_mut(&shid).ok() {
    //     //     shape.set_cstr(Constraint::F);
    //     // }
    // }
    hide_cm_shape(&mut pam);
}

///////////////
/// Settings panel events
fn on_apply_settings_click(pa: RefPA, _event: Event) {
    let mut pam = pa.borrow_mut();

    let width_str = pam.settings_width_input.value();
    let height_str = pam.settings_height_input.value();
    let width: f64 = width_str.parse().unwrap_or(0.0);
    let height: f64 = height_str.parse().unwrap_or(0.0);
    pam.settings_panel
        .style()
        .set_property("display", "none")
        .unwrap();
    pam.modal_backdrop
        .style()
        .set_property("display", "none")
        .unwrap();

    pam.working_area = Vec2 {
        x: width,
        y: height,
    };

    drop(pam);
    resize_area(pa.clone());
    render(pa.clone());
}
fn on_modal_backdrop_click(pa: RefPA, _event: Event) {
    let pam = pa.borrow_mut();
    pam.settings_panel
        .style()
        .set_property("display", "none")
        .unwrap();
    pam.modal_backdrop
        .style()
        .set_property("display", "none")
        .unwrap();
    pam.settings_width_input
        .set_value(&pam.working_area.x.to_string());
    pam.settings_height_input
        .set_value(&pam.working_area.y.to_string());
}

///////////////
// Window events
fn resize_area(pa: RefPA) {
    let mut pam = pa.borrow_mut();
    let (window_width, window_height) = {
        (
            pam.window.inner_width().unwrap().as_f64().unwrap() as u32,
            pam.window.inner_height().unwrap().as_f64().unwrap() as u32,
        )
    };
    let left_panel_width = pam
        .document
        .get_element_by_id("left-panel")
        .unwrap()
        .get_bounding_client_rect()
        .width() as u32;
    let status_bar_height = pam
        .document
        .get_element_by_id("status-bar")
        .unwrap()
        .get_bounding_client_rect()
        .height() as u32;
    let top_menu_height = pam
        .document
        .get_element_by_id("top-menu")
        .unwrap()
        .get_bounding_client_rect()
        .height() as u32;

    let canvas_width = window_width - left_panel_width;
    let canvas_height = window_height - top_menu_height - status_bar_height;

    pam.canvas
        .style()
        .set_property("margin-top", &format!("{}px", top_menu_height))
        .unwrap();
    pam.canvas
        .style()
        .set_property("margin-left", &format!("{}px", left_panel_width))
        .unwrap();
    pam.canvas.set_width(canvas_width);
    pam.canvas.set_height(canvas_height);

    // Calculation starting parameters
    let working_area = pam.working_area;
    let canvas_offset = Vec2 {
        x: (canvas_width as f64 - working_area.x).abs() / 4.,
        y: (canvas_height as f64 - working_area.y).abs() / 3.,
    };
    let dx = canvas_width as f64 / working_area.x / 0.3;
    let dy = canvas_height as f64 / working_area.y / 0.3;
    pam.canvas_offset = canvas_offset;
    pam.global_scale = dx.min(dy);
}
fn on_window_resize(pa: RefPA, _event: Event) {
    resize_area(pa.clone());
    render(pa.clone());
}
fn on_window_click(_pa: RefPA, _event: Event) {
    // let pam = pa.borrow_mut();
    // if let Ok(mouse_event) = event.clone().dyn_into::<MouseEvent>() {
    //     // Not a right-click
    //     if mouse_event.buttons() == 1 {
    //         let target = event.target().unwrap();
    //         let target = target.dyn_into::<web_sys::Node>().unwrap();
    //         if !pam.settings_panel.contains(Some(&target)) {
    //             pam
    //                 .settings_panel
    //                 .style()
    //                 .set_property("display", "none")
    //                 .unwrap();
    //             pam
    //                 .modal_backdrop
    //                 .style()
    //                 .set_property("display", "none")
    //                 .unwrap();
    //         }
    //     }
    // }
}

///////////////
// Icons events
fn on_icon_click(pa: RefPA, event: Event) {
    let mut pam = pa.borrow_mut();
    if let Some(target) = event.target() {
        if let Some(element) = wasm_bindgen::JsCast::dyn_ref::<Element>(&target) {
            if let Some(id) = element.get_attribute("id") {
                if let Some(key) = pam.user_icons.keys().find(|&&k| k == id) {
                    if let Some(icon) = Icons::from_str(key) {
                        pam.icon_selected = icon;
                        pam.csp.on_creation = None;
                        pam.csp.clear_selections_all();
                    }

                    match pam.icon_selected {
                        // Icons::COG => {
                        //     pam.settings_panel
                        //         .style()
                        //         .set_property("display", "block")
                        //         .unwrap();
                        //     pam.modal_backdrop
                        //         .style()
                        //         .set_property("display", "block")
                        //         .unwrap();
                        //     pam.icon_selected = Icons::COG;
                        // }
                        _ => {
                            html_deselect_icons(&pam);
                            html_select_icon(&pam, &id);
                        }
                    }
                }
            }
        }
    }
    drop(pam);
    render(pa.clone());
}
fn on_icon_mouseover(pa: RefPA, event: Event) {
    let pam = pa.borrow_mut();
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
fn on_icon_mouseout(pa: RefPA, _event: Event) {
    pa.borrow_mut()
        .tooltip
        .style()
        .set_property("display", "none")
        .expect("Failed to set display property");
}

///////////////
// Helpers
fn go_to_arrow_tool(pam: &mut RefMut<'_, PlayingArea>) {
    pam.icon_selected = Icons::Arrow;
    html_deselect_icons(&pam);
    html_select_icon(&pam, &Icons::Arrow.as_str());
}
fn html_select_icon(pam: &RefMut<'_, PlayingArea>, name: &str) {
    if let Some(element) = pam.user_icons.get(name).unwrap().clone() {
        if let Ok(html_element) = element.dyn_into::<HtmlElement>() {
            html_element
                .set_attribute("class", "icon icon-selected")
                .expect("Failed to set class attribute");
        }
    }
}
fn html_deselect_icons(pam: &RefMut<'_, PlayingArea>) {
    for (key, oelement) in pam.user_icons.iter() {
        if key != &"icon-cog" {
            if let Some(element) = oelement {
                // let element_cloned = element.clone();
                element
                    .set_attribute("class", "icon")
                    .expect("Failed to set class attribute");
            }
        }
    }
}
fn get_element(document: &Document, element_id: &str) -> Result<Element, JsValue> {
    let element = document
        .get_element_by_id(element_id)
        .ok_or_else(|| JsValue::from_str("should have element on the page"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("should be an HtmlElement"))?;
    Ok(element)
}

///////////////
// Rendering
fn render(pa: RefPA) {
    let pam = pa.borrow_mut();

    // Clear the canvas
    raw_draw_clear_canvas(&pam);
    drop(pam);

    // Then draw all
    if let Err(e) = draw_all(pa.clone()) {
        log!("Error: {}", e);
    }
}
fn draw_all(pa: RefPA) -> Result<(), MyError> {
    draw_grid(pa.clone());
    draw_working_area(pa.clone());
    draw_content(pa.clone())?;
    draw_selection_area(pa.clone());
    Ok(())
}
fn draw_working_area(pa: RefPA) {
    let pam = pa.borrow_mut();
    // Draw working area

    let _wa = pam.working_area;
    // Title
    // cst.push(CTText(Point::new(wa.x / 3., -20.), "Working sheet".into()));

    // let mut shape = ShapeType::new_line(Line::new(pick_pos, pick_pos + (snap_grid, snap_grid)));

    // // Arrows
    // let mut pos = Point::new(0., -10.);
    // prefab::arrow_right(pos, 100., &mut cst);
    // cst.push(CTText(Point::new(40., -20.), "X".into()));

    // pos = Point::new(-10., 0.);
    // prefab::arrow_down(pos, 100., &mut cst);
    // cst.push(CTText(Point::new(-30., 50.), "Y".into()));

    // // Border
    // use ConstructionPattern::*;
    // pos = Point::ZERO;
    // cst.push(CTSegment(NoSelection, pos, pos + (0., wa.y)));
    // cst.push(CTSegment(NoSelection, pos, pos + (wa.x, 0.)));
    // pos = Point::new(wa.x, wa.y);
    // cst.push(CTSegment(NoSelection, pos, pos + (-wa.x, 0.)));
    // cst.push(CTSegment(NoSelection, pos, pos + (0., -wa.y)));

    // draw_shape(&pam, &cst);
}
fn draw_grid(pa: RefPA) {
    let pam = pa.borrow_mut();
    let wa = pam.working_area;
    let w_grid_spacing = pam.working_area_visual_grid;

    use PathEl::*;
    let mut v: Vec<PathEl> = vec![];
    // Vertical grid lines
    let mut wx = 0.;
    while wx <= wa.x {
        v.push(MoveTo(Point::new(wx, 0.)));
        v.push(LineTo(Point::new(wx, wa.y)));
        wx += w_grid_spacing
    }
    // Horizontal grid lines
    let mut wy = 0.;
    while wy <= wa.y {
        v.push(MoveTo(Point::new(0., wy)));
        v.push(LineTo(Point::new(wa.x, wy)));
        wy += w_grid_spacing;
    }
    draw_path(
        &pam,
        BezPath::from_vec(v),
        Layer::Grid,
        Pattern::Grid,
        false,
    );
}
fn draw_content(pa: RefPA) -> Result<(), MyError> {
    let pam = pa.borrow_mut();
    let scale = pam.global_scale;

    for (_cshid, cshape) in pam.csp.iter() {
        let layer = Layer::Worksheet;

        // Shape
        let pattern = if cshape.is_selected() {
            Pattern::Selected
        } else {
            Pattern::Normal
        };
        use COperation::*;
        let filled = match cshape.get_op() {
            Add => true,
            Sub => false,
            And => true,
        };
        draw_path(&pam, cshape.get_shape_path(), layer, pattern, filled);
        // Handles
        for handle in cshape.get_handles().iter() {
            let (pattern, path) = handle.get_path(scale);
            draw_path(&pam, path, layer, pattern, true);
        }
    }

    // Draw cross cursor
    // draw_path(
    //     &pam,
    //     &prefab::cross(&pam.draw_cursor, 10. / scale),
    //     &Layer::Dimension,
    //     &Pattern::Normal,
    //     false,
    // );

    Ok(())
}
fn draw_selection_area(_pa: RefPA) {
    // use ConstructionPattern::*;
    // let pam = pa.borrow_mut();
    // if let Some(sa) = pam.selection_area {
    //     let bl = sa[0];
    //     let tr = sa[1];
    //     if bl.x != tr.x && bl.y != tr.y {
    //         let tl = Point::new(bl.x, tr.y);
    //         let br = Point::new(tr.x, bl.y);
    //         let mut cst = Vec::new();
    //         // cst.push(Move(bl));
    //         cst.push(CTSegment(NoSelection, bl, tl));
    //         cst.push(CTSegment(NoSelection, tl, tr));
    //         cst.push(CTSegment(NoSelection, tr, br));
    //         cst.push(CTSegment(NoSelection, br, bl));
    //         raw_draw(&pam, &cst, ConstructionLayer::SelectionTool);
    //     }
    // }
}
fn draw_path(
    pam: &RefMut<'_, PlayingArea>,
    path: BezPath,
    _layer: Layer,
    pattern: Pattern,
    fill: bool,
) {
    let scale = pam.global_scale;
    let offset = pam.canvas_offset;

    let (stroke_style, stroke_width) = pam.draw_styles.get_styles(pattern);
    let (fill_color, stroke_color) = pam.draw_styles.get_colors(pattern);
    pam.ctx.set_font("20px sans-serif");
    pam.ctx.set_line_dash(stroke_style).unwrap();
    pam.ctx.set_line_width(stroke_width);
    pam.ctx.set_stroke_style(&stroke_color.into());
    pam.ctx.set_fill_style(&fill_color.into());

    pam.ctx.begin_path();

    // if let Some(PathEl::MoveTo(pt)) = path.iter().next() {
    //     let cpt = to_canvas(&pt.to_vec2(), scale, &offset);
    //     pam.ctx.move_to(cpt.x, cpt.y);
    // }
    for cst in path.iter() {
        match cst {
            PathEl::MoveTo(pt) => {
                let cpt = to_canvas(&pt.to_vec2(), scale, &offset);
                pam.ctx.move_to(cpt.x, cpt.y);
            }
            PathEl::LineTo(pt) => {
                let cpt = to_canvas(&pt.to_vec2(), scale, &offset);
                pam.ctx.line_to(cpt.x, cpt.y);
            }
            PathEl::QuadTo(pt1, pt2) => {
                let cpt1 = to_canvas(&pt1.to_vec2(), scale, &offset);
                let cpt2 = to_canvas(&pt2.to_vec2(), scale, &offset);
                pam.ctx.quadratic_curve_to(cpt1.x, cpt1.y, cpt2.x, cpt2.y);
            }
            PathEl::CurveTo(pt1, pt2, pt3) => {
                let cpt1 = to_canvas(&pt1.to_vec2(), scale, &offset);
                let cpt2 = to_canvas(&pt2.to_vec2(), scale, &offset);
                let cpt3 = to_canvas(&pt3.to_vec2(), scale, &offset);
                pam.ctx
                    .bezier_curve_to(cpt1.x, cpt1.y, cpt2.x, cpt2.y, cpt3.x, cpt3.y);
            }
            PathEl::ClosePath => pam.ctx.close_path(),
        }
    }
    if fill {
        pam.ctx.fill();
    }
    pam.ctx.stroke();
}
fn raw_draw_clear_canvas(pam: &RefMut<'_, PlayingArea>) {
    pam.ctx.set_stroke_style(&"#F00".into());
    let background_color = pam.draw_styles.get_background_color();
    pam.ctx.set_fill_style(&background_color.to_string().into());

    pam.ctx.fill();
    let (canvas_width, canvas_height) = { (pam.canvas.width() as f64, pam.canvas.height() as f64) };
    pam.ctx
        .fill_rect(0., 0., canvas_width as f64, canvas_height as f64);
}
