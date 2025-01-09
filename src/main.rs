// #![cfg(not(test))]
// A macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into())
    }
}
pub mod canvas;
pub mod clipboard;
pub mod dimensions;
pub mod dom;
pub mod helpers;
pub mod math;
pub mod pools;
pub mod positions;
pub mod prefab;
pub mod shapes;
pub mod traits;
use crate::dom::*;
use crate::math::*;
use canvas::{CanvasKind, Canvases, DrawStyles, Pattern};
use clipboard::*;
use kurbo::{Size, Vec2};
use pools::{DrawObjects, Pools};
use positions::*;
use shapes::shapes::{BoolOps, MoveShapesAction, ToogleBoolOpsShapesAction};
use shapes::shapes_pool::{AddShapeAction, RemoveShapeAction, ShapesPool};
use std::collections::HashSet;
use std::{
    cell::{RefCell, RefMut},
    rc::Rc,
};
use svg::node::element::path::Data;
use svg::read;
use traits::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    window, Document, Element, Event, FileList, FileReader, HtmlCanvasElement, HtmlElement,
    HtmlInputElement, KeyboardEvent, MouseEvent, WheelEvent, Window,
};

type RefAV = Rc<RefCell<AppVars>>;
type ElementCallback = Box<dyn Fn(RefAV, Event) + 'static>;

#[derive(Default)]
struct KeysStates {
    crtl_pressed: bool,
    shift_pressed: bool,
}

fn main() {
    console_error_panic_hook::set_once();
    let window = window().expect("no global `window` exists");
    create_app_vars(window).expect("Could not access the document");
}

#[allow(dead_code)]
struct AppVars {
    pools: Pools,
    on_creation: DrawObjects, // Shape or Helper

    //
    clipboard: Clipboard,
    undo_redo: UndoRedo,

    // DOM
    window: Window,
    document: Document,
    body: HtmlElement,
    top_menu: HtmlElement,
    left_panel: HtmlElement,
    status_bar: HtmlElement,
    canvases: Canvases,
    user_icons: HashSet<Icons>,
    tooltip: HtmlElement,
    settings_panel: HtmlElement,
    modal_backdrop: HtmlElement,
    apply_settings_button: HtmlElement,
    settings_width_input: HtmlInputElement,
    settings_height_input: HtmlInputElement,

    mouse_position: HtmlElement,
    status_shape: HtmlElement,

    magnet_angle: f64,
    grab_distance: f64,

    icon_selected: Icons,
    selection_area: Option<[Vec2; 2]>,
    keys_states: KeysStates,
    mouse: Mouse,
    pointer: Pointer,

    styles: DrawStyles,
}

///////////////
// Initialization
fn create_app_vars(window: Window) -> Result<(), JsValue> {
    log!("Creating application variables");
    let document = window.document().expect("should have a document on window");
    let document_element = document
        .document_element()
        .ok_or("should have a document element")?;
    let styles = window
        .get_computed_style(&document_element)
        .unwrap()
        .unwrap();
    let body = document.body().expect("should have a body on document");
    let c_draw = document
        .get_element_by_id("mainCanvas")
        .expect("should have mainCanvas on the page")
        .dyn_into::<HtmlCanvasElement>()?;
    let c_grid = document
        .get_element_by_id("gridCanvas")
        .expect("should have gridCanvas on the page")
        .dyn_into::<HtmlCanvasElement>()?;
    let c_back = document
        .get_element_by_id("backgroundCanvas")
        .expect("should have backgroundCanvas on the page")
        .dyn_into::<HtmlCanvasElement>()?;

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
    let mouse_position: HtmlElement = document
        .get_element_by_id("status-mouse-position")
        .expect("should have status-mouse-position on the page")
        .dyn_into()?;
    let status_shape: HtmlElement = document
        .get_element_by_id("status-shape")
        .expect("should have status-viewgrid on the page")
        .dyn_into()?;

    // Load the font

    let canvases = Canvases::new(
        window.clone(),
        c_back,
        c_grid,
        c_draw,
        Size::new(3000., 1500.),
    )?;

    let wa_size = canvases.get_drawing_size();
    settings_width_input.set_value(&wa_size.width.to_string());
    settings_height_input.set_value(&wa_size.height.to_string());

    let mut user_icons: HashSet<Icons> = HashSet::new();
    use Icons::*;
    use IconsShapes::*;
    user_icons.insert(Arrow);
    user_icons.insert(IShapes(Rectangle));
    user_icons.insert(IShapes(RectangleRounded));
    user_icons.insert(IShapes(Disc));
    user_icons.insert(IShapes(Oblong));
    user_icons.insert(IConstruction(IconsConstruction::Point));
    user_icons.insert(IConstruction(IconsConstruction::Line));
    user_icons.insert(IConstruction(IconsConstruction::Circle));

    let left_panel = document
        .get_element_by_id("left-panel")
        .unwrap()
        .dyn_into()?;
    let status_bar = document
        .get_element_by_id("status-bar")
        .unwrap()
        .dyn_into()?;
    let top_menu = document.get_element_by_id("top-menu").unwrap().dyn_into()?;

    let styles = DrawStyles::build(styles)?;

    let app_vars = Rc::new(RefCell::new(AppVars {
        pools: Pools::new(),
        on_creation: DrawObjects::Nope,
        clipboard: Clipboard::new(),
        undo_redo: UndoRedo::new(),
        window,
        document,
        body,
        top_menu,
        left_panel,
        status_bar,
        canvases,
        //
        user_icons,
        tooltip,
        settings_panel,
        modal_backdrop,
        apply_settings_button,
        settings_width_input,
        settings_height_input,
        mouse_position,
        status_shape,

        magnet_angle: 0.05,
        grab_distance: 20.,

        icon_selected: Icons::Arrow,
        selection_area: None,
        keys_states: KeysStates::default(),
        mouse: Mouse::new(),
        pointer: Pointer::new(),

        styles,
    }));

    init_menu(app_vars.clone())?;
    // init_context_menu(app_vars.clone())?;
    init_icons(app_vars.clone())?;
    init_settings_panel(app_vars.clone())?;
    init_status(app_vars.clone())?;
    init_canvas(app_vars.clone())?;
    init_window(app_vars.clone())?;
    resize_canvases(app_vars.clone());

    let av = app_vars.clone();
    let mut avb = av.borrow_mut();
    update_status_bar(&mut avb);
    render_drawing(&mut avb);

    Ok(())
}
fn init_window(av: RefAV) -> Result<(), JsValue> {
    // Resize event
    let pa_cloned1 = av.clone();
    let pa_cloned2 = av.clone();

    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_window_resize(pa_cloned1.clone(), event);
    });

    pa_cloned2
        .borrow_mut()
        .window
        .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())?;

    closure.forget();

    // Click event
    let pa_cloned1 = av.clone();
    let pa_cloned2 = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_window_click(pa_cloned1.clone(), event);
    });
    pa_cloned2
        .borrow_mut()
        .window
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
    closure.forget();

    // Key keydown event
    let pa_cloned1 = av.clone();
    let pa_cloned2 = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_window_keydown(pa_cloned1.clone(), event);
    });
    pa_cloned2
        .borrow_mut()
        .window
        .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())?;
    closure.forget();

    // Key keydup event
    let pa_cloned1 = av.clone();
    let pa_cloned2 = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_window_keyup(pa_cloned1.clone(), event);
    });
    pa_cloned2
        .borrow_mut()
        .window
        .add_event_listener_with_callback("keyup", closure.as_ref().unchecked_ref())?;
    closure.forget();

    Ok(())
}
fn init_settings_panel(av: RefAV) -> Result<(), JsValue> {
    let pam = av.borrow_mut();
    set_callback(
        av.clone(),
        "click".into(),
        &pam.apply_settings_button,
        Box::new(on_apply_settings_click),
    )?;
    set_callback(
        av.clone(),
        "click".into(),
        &pam.modal_backdrop,
        Box::new(on_modal_backdrop_click),
    )?;
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
fn _init_menu(av: RefAV) -> Result<(), JsValue> {
    let pam = av.borrow_mut();
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
        let pa_clone = av.clone();
        // Get the files from the file input element
        if let Some(file_input) = document.get_element_by_id("file-input") {
            let file_input: HtmlInputElement = file_input.dyn_into().unwrap();
            let files = js_sys::Reflect::get(&file_input.into(), &"files".into())
                .unwrap()
                .dyn_into::<FileList>()
                .unwrap();
            if let Some(file) = files.get(0) {
                let file_name = file.name();
                log!("File selected: {:?}", file_name);
                let file_reader = FileReader::new().unwrap();

                let on_load = Closure::wrap(Box::new(move |event: Event| {
                    let target = event.target().unwrap();
                    let file_reader: FileReader = target.dyn_into().unwrap();
                    let result = file_reader.result().unwrap();
                    if let Some(content) = result.as_string() {
                        convert_svg_to_shapes(pa_clone.clone(), content);
                        drop(pa_clone.borrow_mut());
                        // render_drawing(pa_clone.clone());
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

fn init_status(av: RefAV) -> Result<(), JsValue> {
    let pam = av.borrow_mut();
    let _document = pam.document.clone();

    Ok(())
}
fn set_callback(
    av: RefAV,
    event_str: String,
    element: &Element,
    callbacka: ElementCallback,
) -> Result<(), JsValue> {
    let event_str_cloned = event_str.clone();

    let callback = Box::new(move |av: RefAV, e: Event| {
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

#[allow(unused_variables)]
fn update(avb: &mut RefMut<'_, AppVars>) -> Result<(), MyError> {
    use MouseState::*;
    let grab = avb.grab_distance / avb.canvases.get_drawing_scale();
    let _magnet_angle = avb.magnet_angle;
    let icon_selected = avb.icon_selected.clone();

    let pointer_pos = avb.mouse.get_draw_pos();
    avb.pointer.set_pos(pointer_pos);
    let shift_pressed = avb.keys_states.shift_pressed;

    match icon_selected {
        Icons::Arrow => match avb.mouse.get_mouse_state() {
            LeftDown(cursor_pos) => {
                // Always clear selection before proceeding
                avb.pools.clear_all_hs();

                if avb.clipboard.is_paste_empty() {
                    avb.pools.hs_objects_in_order(cursor_pos, grab, HS::Select);
                    if avb.keys_states.crtl_pressed {
                        avb.pools.select_all_shapes_connected();
                    }
                } else {
                    // Add the pasted object to the pool
                    if let Some(clip_item) = avb.clipboard.get_paste().cloned() {
                        match &clip_item {
                            ClipboardItem::Shapes((shapes, ..)) => {
                                let new_shapes = avb.pools.sh.duplicate_shapes(shapes.clone());
                                // Push the PasteAction to the undo/redo system
                                avb.undo_redo.push(Box::new(PasteAction {
                                    clip_item: ClipboardItem::Shapes((new_shapes, Vec2::ZERO)),
                                }));
                            }
                            ClipboardItem::Helpers((helpers, ..)) => {
                                let new_helpers = avb.pools.hp.duplicate_helpers(helpers.clone());
                                // Push the PasteAction to the undo/redo system
                                avb.undo_redo.push(Box::new(PasteAction {
                                    clip_item: ClipboardItem::Helpers((new_helpers, Vec2::ZERO)),
                                }));
                            }
                        }
                        avb.clipboard.clear_paste();
                    }
                }
                avb.pools.save_vars();
                Ok(())
            }
            LeftDownMove(pos_dwn, cursor_pos) => {
                let shapes_selected = avb.pools.sh.get_hs(HS::Select);
                if shapes_selected.len() > 0 {
                    avb.pools.sh.move_positions(
                        shapes_selected,
                        pos_dwn,
                        cursor_pos,
                        shift_pressed,
                    );
                } else {
                    if let Some((shid, _)) = avb.pools.sh.get_first_selected_modifier_vars() {
                        avb.pools
                            .sh
                            .move_modifier(shid, pos_dwn, cursor_pos, shift_pressed);
                    }
                }
                Ok(())
            }
            LeftUp(_cursor_pos) => {
                // Push the MoveShapeAction to the undo/redo system
                let shapes_moved = avb.pools.sh.get_hs_vars(HS::Select);
                if shapes_moved.len() > 0 {
                    avb.undo_redo.push(Box::new(MoveShapesAction {
                        shids_vars: shapes_moved,
                    }));
                } else {
                    if let Some(shid_vars) = avb.pools.sh.get_first_selected_modifier_vars() {
                        avb.undo_redo.push(Box::new(MoveShapesAction {
                            shids_vars: vec![shid_vars],
                        }));
                    }
                }
                avb.pools.sh.recalc_full_segs();
                Ok(())
            }
            LeftUpMove(_, cursor_pos) | RightUpMove(_, cursor_pos) => {
                if !avb.clipboard.is_paste_empty() {
                    avb.clipboard.move_paste(cursor_pos);
                } else {
                    avb.pools
                        .hs_objects_in_order(cursor_pos, grab, HS::Highlight);
                }
                Ok(())
            }
            RightDown(_) => {
                avb.clipboard.clear();
                avb.pools.sh.set_hs(false, HS::Select);
                avb.canvases.save_drawing_offset();
                Ok(())
            }
            RightDownMove(pos_dwn, cursor_pos) => {
                avb.canvases.move_drawing_offset(pos_dwn, cursor_pos);
                draw_grid_and_rules(avb);
                Ok(())
            }
            _ => Ok(()),
        },
        Icons::IShapes(ishape) => {
            match avb.mouse.get_mouse_state() {
                LeftDown(pos) => {
                    if let Some(mut shape) = avb.on_creation.get_shape_into() {
                        // Minimum size was not reached
                        if !shape.get_kind().good_size() {
                            log!("Shape too small");
                        } else {
                            // A. We were drawing a new shape, finish all
                            shape.get_kind_mut().set_hs(false, HS::Select);
                            shape.get_kind_mut().set_hs_modifiers(false, HS::Select);
                            avb.pools.sh.add_shape(shape.clone());
                            // Push the AddShapeAction to the undo/redo system
                            avb.undo_redo.push(Box::new(AddShapeAction {
                                shape: shape.clone(),
                            }));
                            //
                            avb.on_creation = DrawObjects::Nope;
                            avb.pools.sh.recalc_full_segs();
                        }
                        Ok(())
                    } else {
                        // B. We start drawing a new shape
                        avb.on_creation.set_shape(ShapesPool::new_shape(
                            ishape,
                            pos,
                            pos,
                            BoolOps::Union,
                        ));
                        Ok(())
                    }
                }
                LeftDownMove(pos_dwn, cursor_pos)
                | LeftUpMove(pos_dwn, cursor_pos)
                | RightUpMove(pos_dwn, cursor_pos) => {
                    if let Some(shape) = avb.on_creation.get_shape_mut() {
                        if shape.get_kind().get_hs_modifiers(HS::Select) {
                            shape
                                .get_kind_mut()
                                .move_modifier(pos_dwn, cursor_pos, false);
                        }
                    }
                    Ok(())
                }
                RightDown(_) => {
                    avb.on_creation = DrawObjects::Nope;
                    go_to_arrow_tool(avb);
                    Ok(())
                }
                _ => Ok(()),
            }
        }
        Icons::IConstruction(iconstruct) => match avb.mouse.get_mouse_state() {
            LeftDown(pos) => Ok(()),
            LeftDownMove(pos_dwn, cursor_pos)
            | LeftUpMove(pos_dwn, cursor_pos)
            | RightUpMove(pos_dwn, cursor_pos) => Ok(()),
            RightDown(_) => {
                avb.on_creation = DrawObjects::Nope;
                go_to_arrow_tool(avb);
                Ok(())
            }
            _ => Ok(()),
        },
        _ => Ok(()),
    }
}

fn update_status_bar(avb: &mut RefMut<'_, AppVars>) {
    //Display: update mouse world position
    let cp = avb.pointer.get_pos();

    avb.mouse_position
        .set_text_content(Some(&format!("( {:.1} , {:.1} )", cp.x, cp.y)));
}
fn on_mouse_move(av: RefAV, event: Event) {
    let mut pam = av.borrow_mut();
    let drawing_offset = pam.canvases.get_drawing_offset();
    let drawing_scale = pam.canvases.get_drawing_scale();
    let offset_start_x = get_element_width(&pam.left_panel);
    let offset_start_y = get_element_height(&pam.top_menu);
    pam.mouse.update_mouse(
        offset_start_x,
        offset_start_y,
        drawing_offset,
        drawing_scale,
        &event,
        SystemMouse::Move,
    );
    if let Some(e) = update(&mut pam).err() {
        log!("ERROR: {}", e);
    }
    update_status_bar(&mut pam);
    render_drawing(&mut pam);

    drop(pam);
}
fn on_mouse_down(av: RefAV, event: Event) {
    let mut pam = av.borrow_mut();
    let drawing_offset = pam.canvases.get_drawing_offset();
    let drawing_scale = pam.canvases.get_drawing_scale();
    let offset_start_x = get_element_width(&pam.left_panel);
    let offset_start_y = get_element_height(&pam.top_menu);
    pam.mouse.update_mouse(
        offset_start_x,
        offset_start_y,
        drawing_offset,
        drawing_scale,
        &event,
        SystemMouse::Down,
    );
    // Hide the context menu when clicking elsewhere
    display_html_element(DOMElements::ContextMenuShape, false);

    if let Some(e) = update(&mut pam).err() {
        log!("ERROR: {}", e);
    }
    // update_status_bar(&mut pam);
    render_drawing(&mut pam);
    drop(pam);
}
fn on_mouse_up(av: RefAV, event: Event) {
    let mut pam = av.borrow_mut();
    let drawing_offset = pam.canvases.get_drawing_offset();
    let drawing_scale = pam.canvases.get_drawing_scale();
    let offset_start_x = get_element_width(&pam.left_panel);
    let offset_start_y = get_element_height(&pam.top_menu);
    pam.mouse.update_mouse(
        offset_start_x,
        offset_start_y,
        drawing_offset,
        drawing_scale,
        &event,
        SystemMouse::Up,
    );

    if let Some(e) = update(&mut pam).err() {
        log!("ERROR: {}", e);
    }
    update_status_bar(&mut pam);
    render_drawing(&mut pam);
    drop(pam);
}
fn on_mouse_wheel(av: RefAV, event: Event) {
    if let Ok(wheel_event) = event.dyn_into::<WheelEvent>() {
        wheel_event.prevent_default();
        let mut pam = av.borrow_mut();

        let zoom_factor = 0.04;
        let old_draw_scale = pam.canvases.get_drawing_scale();
        let old_draw_offset = pam.canvases.get_drawing_offset();

        let new_scale = if wheel_event.delta_y() < 0. {
            (old_draw_scale * (1.0 + zoom_factor)).min(10.0) // Zoom in
        } else {
            (old_draw_scale / (1.0 + zoom_factor)).max(0.5) // Zoom out
        };
        pam.canvases.set_drawing_scale(new_scale);

        let canvas_pos = pam.mouse.get_canvas_pos();
        let old_draw_offset_rel = canvas_pos - old_draw_offset;
        let new_draw_offset = canvas_pos - old_draw_offset_rel * (new_scale / old_draw_scale);

        pam.canvases.set_drawing_offset(new_draw_offset);

        draw_grid_and_rules(&mut pam);
        render_drawing(&mut pam);
        drop(pam);
    }
}
fn on_mouse_enter(av: RefAV, _event: Event) {
    let mut avb = av.borrow_mut();
    avb.pointer.set_active(true);
    render_drawing(&mut avb);
    drop(avb);
}
fn on_mouse_leave(av: RefAV, _event: Event) {
    let mut avb = av.borrow_mut();
    avb.pointer.set_active(false);
    render_drawing(&mut avb);
    drop(avb);
}

fn on_context_menu(_av: RefAV, event: Event) {
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
/// Settings panel events
fn on_apply_settings_click(av: RefAV, _event: Event) {
    let mut pam = av.borrow_mut();
    let width_str = pam.settings_width_input.value();
    let height_str = pam.settings_height_input.value();
    let _width: f64 = width_str.parse().unwrap_or(0.0);
    let _height: f64 = height_str.parse().unwrap_or(0.0);
    pam.settings_panel
        .style()
        .set_property("display", "none")
        .unwrap();
    pam.modal_backdrop
        .style()
        .set_property("display", "none")
        .unwrap();

    render_drawing(&mut pam);
    drop(pam);
}
fn on_modal_backdrop_click(av: RefAV, _event: Event) {
    let pam = av.borrow_mut();
    pam.settings_panel
        .style()
        .set_property("display", "none")
        .unwrap();
    pam.modal_backdrop
        .style()
        .set_property("display", "none")
        .unwrap();
    pam.settings_width_input
        .set_value(&pam.canvases.get_drawing_size().width.to_string());
    pam.settings_height_input
        .set_value(&pam.canvases.get_drawing_size().height.to_string());
}

///////////////
// Window events
fn resize_canvases(av: RefAV) {
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
    let status_bar_height = get_element_height(&pam.status_bar);
    let width = window_width - offset_start_x;
    let height = window_height - offset_start_y - status_bar_height;
    // log!("resize sizes: ({},{})", width, height);
    pam.canvases.resize_canvases(width, height);

    draw_grid_and_rules(&mut pam);
    render_drawing(&mut pam);
    drop(pam);
}
fn on_window_resize(av: RefAV, _event: Event) {
    resize_canvases(av.clone());
    let mut pam = av.borrow_mut();
    render_drawing(&mut pam);
    drop(pam);
}
fn on_window_click(_pa: RefAV, _event: Event) {}
fn on_window_keydown(av: RefAV, event: Event) {
    event.prevent_default();
    if let Ok(keyboard_event) = event.dyn_into::<KeyboardEvent>() {
        let mut avb = av.borrow_mut();

        if keyboard_event.key() == "Control" || keyboard_event.key() == "Meta" {
            log!("control pressed");
            avb.keys_states.crtl_pressed = true;
        }
        if keyboard_event.key() == "Shift" {
            log!("shift pressed");
            avb.keys_states.shift_pressed = true;
        }
        if keyboard_event.key() == "Delete" || keyboard_event.key() == "Backspace" {
            let shapes_deleted = avb.pools.sh.delete_shapes_selected();
            // Push the DeleteShapesAction to the undo/redo system
            avb.undo_redo.push(Box::new(RemoveShapeAction {
                shapes: shapes_deleted,
            }));
            avb.pools.sh.recalc_full_segs();
        }
        if keyboard_event.key() == "Escape" {
            avb.on_creation = DrawObjects::Nope;
            go_to_arrow_tool(&mut avb);
        }

        // Copy and paste
        if keyboard_event.key() == "c" {
            if avb.keys_states.crtl_pressed {
                log!("ctrl-c pressed");
                if let Icons::Arrow = avb.icon_selected.clone() {
                    if let DrawObjects::Nope = avb.on_creation {
                        let cursor_pos = avb.mouse.get_draw_pos();
                        let shapes_selected = avb.pools.sh.get_hs(HS::Select);
                        let helpers_selected = avb.pools.hp.get_hs(HS::Select);

                        if shapes_selected.len() > 0 {
                            let mut to_copy = vec![];
                            for shid in shapes_selected {
                                if let Some(shape) = avb.pools.sh.get_shape(shid) {
                                    to_copy.push(shape.clone());
                                }
                            }
                            avb.clipboard.copy_shapes(to_copy, cursor_pos);
                        } else {
                            if helpers_selected.len() > 0 {
                                let mut to_copy = vec![];
                                for shid in helpers_selected {
                                    if let Some(helper) = avb.pools.hp.get_helper(shid) {
                                        to_copy.push(helper.clone());
                                    }
                                }
                                avb.clipboard.copy_helpers(to_copy, cursor_pos);
                            }
                        }
                    }
                }
            }
        }
        if keyboard_event.key() == "v" {
            let crtl_pressed = avb.keys_states.crtl_pressed;
            if crtl_pressed {
                log!("ctrl-v pressed");
                let icon_selected = avb.icon_selected.clone();
                if let Icons::Arrow = icon_selected {
                    let cursor_pos = avb.mouse.get_draw_pos();
                    avb.clipboard.paste_item(cursor_pos);
                    // Clear actual selection
                    avb.pools.sh.set_hs(false, HS::Select);
                }
            }
        }

        // Undo and Redo
        if keyboard_event.key() == "z" {
            if avb.keys_states.crtl_pressed {
                log!("ctrl-z pressed");
                // Temporarily take ownership of `undo_redo`
                let mut undo_redo = std::mem::take(&mut avb.undo_redo);
                // Perform undo operation
                undo_redo.undo(&mut avb.pools);
                // Put `undo_redo` back into `avb`
                avb.undo_redo = undo_redo;
                // Recalculate the full segments
                avb.pools.sh.recalc_full_segs();
            }
        }
        if keyboard_event.key() == "Z" || keyboard_event.key() == "y" {
            if avb.keys_states.crtl_pressed {
                log!("ctrl-Z pressed or ctrl-y pressed");
                // Temporarily take ownership of `undo_redo`
                let mut undo_redo = std::mem::take(&mut avb.undo_redo);
                // Perform redo operation
                undo_redo.redo(&mut avb.pools);
                // Put `undo_redo` back into `avb`
                avb.undo_redo = undo_redo;
                // Recalculate the full segments
                avb.pools.sh.recalc_full_segs();
            }
        }

        // Toggle boolean operation
        if keyboard_event.key() == "t" {
            if let DrawObjects::Nope = avb.on_creation {
                let mut o_shid = None;
                // let highlighted = avb.pool.get_hs(HS::Highlight);
                // if highlighted.len() == 1 {
                //     if let Some(shape) = avb.pool.get_shape_mut(highlighted[0]) {
                //         o_shid = Some((highlighted[0], shape.get_boolean_op()));
                //     }
                // } else {
                let selected = avb.pools.sh.get_hs(HS::Select);
                if selected.len() == 1 {
                    if let Some(shape) = avb.pools.sh.get_shape_mut(selected[0]) {
                        o_shid = Some((selected[0], shape.get_boolean_op()));
                    }
                }
                // }
                if let Some((shid, bool_ops)) = o_shid {
                    // Push the ToogleBoolOpsShapesAction to the undo/redo system
                    avb.undo_redo.push(Box::new(ToogleBoolOpsShapesAction {
                        shid_toogle: (shid, bool_ops),
                    }));
                    if let Some(shape) = avb.pools.sh.get_shape_mut(shid) {
                        // Do the actual toogle
                        shape.toggle_boolean_op();
                    }
                    avb.pools.sh.recalc_full_segs();
                }
            }
        }
        if keyboard_event.key() == " " {}

        render_drawing(&mut avb);
        drop(avb);
    }
}
fn on_window_keyup(av: RefAV, event: Event) {
    if let Ok(keyboard_event) = event.dyn_into::<KeyboardEvent>() {
        let mut pam = av.borrow_mut();
        if keyboard_event.key() == "Control" || keyboard_event.key() == "Meta" {
            log!("ctrl released");
            pam.keys_states.crtl_pressed = false;
        }
        if keyboard_event.key() == "Shift" {
            log!("shift released");
            pam.keys_states.shift_pressed = false;
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
                    avb.on_creation = DrawObjects::Nope;
                    avb.pools.sh.set_hs(false, HS::Select);
                    avb.pools.sh.set_hs_modifiers(false, HS::Select);
                    // avb.pool.set_hors_centers(false, HighLightOrSelect::Select);

                    avb.user_icons
                        .iter()
                        .for_each(|icon| html_deselect_icons(*icon));
                    html_select_icon(icon);
                }
            }
        }
    }
    render_drawing(&mut avb);
    drop(avb);
}
fn on_icon_mouseover(av: RefAV, event: Event) {
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
fn on_icon_mouseout(av: RefAV, _event: Event) {
    av.borrow_mut()
        .tooltip
        .style()
        .set_property("display", "none")
        .expect("Failed to set display property");
}

///////////////
// Helpers
fn go_to_arrow_tool(avb: &mut RefMut<'_, AppVars>) {
    avb.icon_selected = Icons::Arrow;
    avb.user_icons
        .iter()
        .for_each(|icon| html_deselect_icons(*icon));
    html_select_icon(Icons::Arrow);
}
fn html_select_icon(icon: Icons) {
    if let Some(html_element) = icon.get_html_element() {
        html_element
            .set_attribute("class", "icon icon-selected")
            .expect("Failed to set class attribute");
    }
}
fn html_deselect_icons(icon: Icons) {
    if let Some(html_element) = icon.get_html_element() {
        html_element
            .set_attribute("class", "icon")
            .expect("Failed to set class attribute");
    }
}

///////////////
// Rendering
fn draw_grid_and_rules(avb: &mut RefMut<'_, AppVars>) {
    avb.canvases.draw_grid_and_rules();
}
fn render_drawing(avb: &mut RefMut<'_, AppVars>) {
    // Get the Performance API
    // let performance = window().unwrap().performance().unwrap();
    // let start_time = performance.now();

    let _scale = avb.canvases.get_drawing_scale();
    avb.canvases.clear_main_canvas();

    // Draw pointer
    if avb.pointer.is_active() {
        avb.canvases.draw_pointer(avb.pointer.get_pos());
    }

    // Draw the final contour shapes
    let full_segs = avb.pools.sh.get_full_segs();
    avb.canvases.draw_closed_path(
        &CanvasKind::Draw,
        full_segs,
        Pattern::ComposedNormal(true),
        vec![],
    );

    // SHAPES: Draw the outline of every shape
    for shape in avb.pools.sh.values() {
        avb.canvases
            .draw_path(&CanvasKind::Draw, shape.get_paths_and_patterns(), vec![]);
    }

    // SHAPES: Draw the modifiers points
    for shape in avb.pools.sh.values() {
        avb.canvases.draw_path(
            &CanvasKind::Draw,
            shape.get_kind().get_modifiers_paths(),
            vec![],
        );
    }

    // SHAPES: Draw dimensions
    for shape in avb.pools.sh.values() {
        if shape.get_kind().get_hs(HS::Select)
            || shape.get_kind().get_hs(HS::Highlight)
            || shape.get_kind().get_hs_modifiers(HS::Select)
            || shape.get_kind().get_hs_modifiers(HS::Highlight)
        {
            let (path, texts) = shape.get_kind().get_dimensions_paths();
            avb.canvases.draw_path(&CanvasKind::Draw, path, texts);
        }
    }

    // CONSTRUCTORS: Draw the outline of every shape
    for helper in avb.pools.hp.values() {
        avb.canvases
            .draw_path(&CanvasKind::Draw, helper.get_paths_and_patterns(), vec![]);
    }

    // Draw the clipboard item if any
    if let Some(item) = avb.clipboard.get_paste() {
        match item {
            ClipboardItem::Shapes((shapes, _)) => {
                for shape in shapes {
                    avb.canvases.draw_path(
                        &CanvasKind::Draw,
                        shape.get_paths_and_patterns(),
                        vec![],
                    );
                }
            }
            ClipboardItem::Helpers((helpers, _)) => {
                for helper in helpers {
                    avb.canvases.draw_path(
                        &CanvasKind::Draw,
                        helper.get_paths_and_patterns(),
                        vec![],
                    );
                }
            }
        }
    }

    // Draw the on_creation object if any
    if let Some(shape) = &avb.on_creation.get_shape_into() {
        avb.canvases
            .draw_path(&CanvasKind::Draw, shape.get_paths_and_patterns(), vec![]);
        // With dimensions
        let (path, texts) = shape.get_kind().get_dimensions_paths();
        avb.canvases.draw_path(&CanvasKind::Draw, path, texts);
    } else {
        if let Some(helper) = &avb.on_creation.get_helper() {
            avb.canvases
                .draw_path(&CanvasKind::Draw, helper.get_paths_and_patterns(), vec![]);
        }
    }

    // let end_time = performance.now();
    // log!("Rendering time: {:.2} ms", end_time - start_time);
}
