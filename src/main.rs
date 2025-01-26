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
pub mod primitives;
pub mod shapes;
pub mod traits;
use crate::dom::*;
use crate::math::*;
use canvas::CanvasText;
use canvas::CanvasTextConfig;
use canvas::TextAlign;
use canvas::TextPos;
use canvas::{CanvasKind, Canvases, DrawStyles, Pattern};
use clipboard::*;
use helpers::helpers_pool::AddHelperAction;
use helpers::helpers_pool::DeleteHelperAction;
use helpers::helpers_pool::HelpersPool;
use kurbo::{Size, Vec2};
use pools::PoolsFunctions;
use pools::HS;
use pools::{DrawObjects, Pools};
use positions::*;
use prefab::modifiers_path;
use primitives::primitives::GetPrimitiveState;
use primitives::primitives::VertexProperty;
use shapes::shapes::BSKind;
use shapes::shapes::{BoolOps, ToogleBoolOpsShapesAction};
use shapes::shapes_pool::BSid;
use shapes::shapes_pool::{AddShapeAction, DeleteShapeAction, ShapesPool};
use std::collections::HashSet;
use std::str::FromStr;
use std::{
    cell::{RefCell, RefMut},
    rc::Rc,
};
use svg::node::element::path::Data;
use svg::read;
use traits::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{
    window, Document, Element, Event, FileReader, HtmlCanvasElement, HtmlElement, HtmlInputElement,
    KeyboardEvent, MouseEvent, WheelEvent, Window,
};

type RefAV = Rc<RefCell<AppVars>>;
type ElementCallback = Box<dyn Fn(RefAV, Event) + 'static>;

#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct KeysStates {
    crtl_cmd_pressed: bool,
    shift_pressed: bool,
    alt_pressed: bool,
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
    canvases: Canvases,
    user_icons: HashSet<Icons>,
    tooltip: HtmlElement,
    settings_panel: HtmlElement,
    modal_backdrop: HtmlElement,
    apply_settings_button: HtmlElement,
    settings_width_input: HtmlInputElement,
    settings_height_input: HtmlInputElement,

    icon_selected: Icons,
    selection_area: Option<[Vec2; 2]>,
    keys_states: KeysStates,
    mouse: Mouse,
    pointer: Pointer,

    styles: DrawStyles,
}

impl AppVars {
    fn cancel_entity_creation(&mut self) {
        self.on_creation = DrawObjects::Nope;
        self.go_to_arrow_tool();
    }
    fn copy_entity(&mut self) {
        if let Icons::Arrow = self.icon_selected.clone() {
            if let DrawObjects::Nope = self.on_creation {
                let shapes_selected = self.pools.shapes.get_state(HS::Select);
                let helpers_selected = self.pools.helpers.get_state(HS::Select);
                let mut pointer = self.pointer.clone();

                if shapes_selected.len() > 0 {
                    let mut to_copy = vec![];
                    for shid in shapes_selected.iter() {
                        if let Some(shape) = self.pools.shapes.get(*shid) {
                            pointer.set_pos(shape.get_kind().get_position());
                            to_copy.push(shape.clone());
                        }
                    }
                    self.clipboard.copy_shapes(to_copy, pointer.pos());
                } else {
                    if helpers_selected.len() > 0 {
                        let mut to_copy = vec![];
                        for shid in helpers_selected {
                            if let Some(helper) = self.pools.helpers.get(shid) {
                                to_copy.push(helper.clone());
                            }
                        }
                        self.clipboard.copy_helpers(to_copy, pointer.pos());
                    }
                }
            }
        }
    }
    fn paste_entity(&mut self) {
        let mut pointer = self.pointer;
        let icon_selected = self.icon_selected.clone();
        if let Icons::Arrow = icon_selected {
            self.clipboard.paste_item(&mut pointer);
            // Clear actual selection
            self.pools.shapes.set_state(false, HS::Select);
        }
        self.pointer = pointer;
    }
    fn delete_entity(&mut self) {
        if let Some(shapes_deleted) = self.pools.delete_shapes_selection() {
            // Push the DeleteShapesAction to the undo/redo system
            self.undo_redo.push(Box::new(DeleteShapeAction {
                shapes: shapes_deleted,
            }));
            self.pools.shapes.recalc_full_segs();
        }
        if let Some(helpers_deleted) = self.pools.delete_helpers_selection() {
            // Push the DeleteShapesAction to the undo/redo system
            self.undo_redo.push(Box::new(DeleteHelperAction {
                helpers: helpers_deleted,
            }));
            self.pools.shapes.recalc_full_segs();
        }
        self.pools.shapes.recalc_full_segs();
    }
    fn next_snap(&mut self) {
        let snap = &self.pointer.get_snap();
        self.pointer.set_snap(match snap {
            SnapValue::Snap1 => SnapValue::Snap5,
            SnapValue::Snap5 => SnapValue::Snap10,
            SnapValue::Snap10 => SnapValue::Snap1,
        });
    }
    fn toogle_boolean_op(&mut self) {
        if let DrawObjects::Nope = self.on_creation {
            let mut o_shid = None;
            let selected = self.pools.shapes.get_state(HS::Select);
            if selected.len() == 1 {
                if let Some(shape) = self.pools.shapes.get_mut(selected[0]) {
                    o_shid = Some((selected[0], shape.get_boolean_op()));
                }
            }
            if let Some((shid, bool_ops)) = o_shid {
                // Push the ToogleBoolOpsShapesAction to the undo/redo system
                self.undo_redo.push(Box::new(ToogleBoolOpsShapesAction {
                    shid_toogle: (shid, bool_ops),
                }));
                if let Some(shape) = self.pools.shapes.get_mut(shid) {
                    shape.toggle_boolean_op();
                }
                self.pools.shapes.recalc_full_segs();
            }
        }
    }
    fn toogle_primitive_prop(&mut self) {
        use GetPrimitiveState::*;
        let o_shid = self
            .pools
            .shapes
            .get_first_selected_modifier_vars()
            .and_then(|(shid, _)| Some(shid));
        if let Some(shid) = o_shid {
            let mut o_pos = None;
            self.pools
                .shapes
                .get_mut(shid)
                .and_then(|shape| {
                    if let BSKind::Custom(shape_custom) = shape.get_kind_mut() {
                        Some(shape_custom)
                    } else {
                        None
                    }
                })
                .map(|shape_custom| {
                    let prims = shape_custom.get_prims_mut();

                    prims.iter_mut().for_each(|prim| {
                        if prim.get_state(IsSelected).is_some() {
                            o_pos = Some(prim.toogle());
                        }
                    });
                    shape_custom.update_polygon();
                });
            if let Some(_pos) = o_pos {
                // avb.pointer.set_pos(pos);
            };
            self.pools.recalc_full_segs();
        }
    }
    fn change_shape_custom(&mut self, shid: BSid) {
        let shift_pressed = self.keys_states.shift_pressed;
        if let Some(shape) = self.pools.shapes.get_mut(shid) {
            if let BSKind::Custom(shape_custom) = shape.get_kind_mut() {
                for primitive in shape_custom.get_prims_mut() {
                    // A) Change first edge selected and return
                    if primitive.get_state(GetPrimitiveState::IsSelected).is_some() {
                        if shift_pressed {
                            primitive.set_prim_kind_prev();
                        } else {
                            primitive.set_prim_kind_next();
                        }
                        break;
                    }
                    // B) Change first vertex selected and return
                    if primitive
                        .get_state(GetPrimitiveState::IsStartSelected)
                        .is_some()
                    {
                        primitive.toogle_start_modifier();
                        let start_modifier = primitive.get_start_modifier();
                        log!("switched to {:?}", start_modifier);
                        if shift_pressed {
                            // Change all primitives modifiers
                            for prim in shape_custom.get_prims_mut() {
                                prim.set_start_modifier(start_modifier);
                            }
                        }
                        break;
                    }
                }
                shape_custom.update_polygon();
                self.pools.shapes.recalc_full_segs();
            }
        }
    }
    fn undo(&mut self) {
        // Temporarily take ownership of `undo_redo`
        let mut undo_redo = std::mem::take(&mut self.undo_redo);
        // Perform undo operation
        undo_redo.undo(&mut self.pools);
        // Put `undo_redo` back into `avb`
        self.undo_redo = undo_redo;
        // Recalculate the full segments
        self.pools.shapes.recalc_full_segs();
    }
    fn redo(&mut self) {
        // Temporarily take ownership of `undo_redo`
        let mut undo_redo = std::mem::take(&mut self.undo_redo);
        // Perform redo operation
        undo_redo.redo(&mut self.pools);
        // Put `undo_redo` back into `avb`
        self.undo_redo = undo_redo;
        // Recalculate the full segments
        self.pools.shapes.recalc_full_segs();
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
    user_icons.insert(IShapes(Disc));
    user_icons.insert(IShapes(Custom));
    user_icons.insert(IHelpers(IconsConstruction::Line));
    user_icons.insert(IHelpers(IconsConstruction::Circle));

    let left_panel = document
        .get_element_by_id("left-panel")
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
        canvases,
        //
        user_icons,
        tooltip,
        settings_panel,
        modal_backdrop,
        apply_settings_button,
        settings_width_input,
        settings_height_input,

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

    reset_origin(&mut avb);
    update_informations(&mut avb);
    draw_grid_and_rules(&mut avb);
    render_drawing(&mut avb);

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
    let icon_selected = avb.icon_selected.clone();
    let keys_states = avb.keys_states;

    let mut pointer = avb.pointer;
    pointer.set_draw_scale(avb.canvases.get_drawing_scale());

    use GetEntityState::*;
    use SetEntityState::*;
    match icon_selected {
        Icons::Arrow => match avb.mouse.get_mouse_state() {
            LeftDown(mouse_pos_down) => {
                pointer.set_pos(mouse_pos_down);
                pointer.save_pos();
                avb.pools.magnet_to_helpers(&mut pointer, keys_states);

                // Always clear selection before proceeding
                avb.pools.clear_all_hs();

                if avb.clipboard.is_paste_empty() {
                    let pointer_pos_saved = pointer.pos_saved();
                    avb.pools
                        .set_objects_states_in_order(&mut pointer, keys_states, HS::Select);
                    if avb.keys_states.crtl_cmd_pressed {
                        avb.pools.select_all_shapes_connected();
                        pointer.set_pos(pointer_pos_saved);
                    }
                } else {
                    // User has clicked for pasting the object on the canvas
                    if let Some(clip_item) = avb.clipboard.get_paste().cloned() {
                        // Add the pasted object to the pool
                        let paste = avb.pools.paste_to_pool(clip_item);
                        // Push the PasteAction to the undo/redo system
                        avb.undo_redo.push(paste);
                        avb.clipboard.clear_paste();
                    }
                }
                avb.pools.save_vars();
            }
            LeftDownMove(mouse_pos_down, mouse_pos) => {
                pointer.set_pos(mouse_pos);
                avb.pools.magnet_to_helpers(&mut pointer, keys_states);

                _ = avb.pools.move_objects(&mut pointer, keys_states);
            }
            LeftUp(mouse_pos_up) => {
                pointer.set_pos(mouse_pos_up);
                pointer.save_pos();
                avb.pools.magnet_to_helpers(&mut pointer, keys_states);

                // Push the MoveAction to the undo/redo system
                if let Some(move_action) = avb.pools.get_move_action() {
                    avb.undo_redo.push(move_action);
                }
                // User has finished moving the objects, recalculate the full segments
                avb.pools.recalc_full_segs();
                avb.pools.helpers.create_helpers_magnet_points();
            }
            LeftUpMove(mouse_pos_up, mouse_pos) | RightUpMove(mouse_pos_up, mouse_pos) => {
                pointer.set_pos(mouse_pos);
                avb.pools.magnet_to_helpers(&mut pointer, keys_states);
                if !avb.clipboard.is_paste_empty() {
                    avb.clipboard.move_paste(&mut pointer);
                } else {
                    avb.pools
                        .set_objects_states_in_order(&mut pointer, keys_states, HS::Highlight);
                }
            }
            RightDown(_) => {
                avb.clipboard.clear();
                avb.pools.set_objects_state(false, HS::Select);
                avb.canvases.save_drawing_offset();
            }
            RightDownMove(mouse_pos_down, mouse_pos) => {
                avb.canvases.move_drawing_offset(mouse_pos_down, mouse_pos);
                draw_grid_and_rules(avb);
            }
            RightUp(mouse_pos_up) => {
                pointer.set_pos(mouse_pos_up);
                pointer.save_pos();
            }
            _ => (),
        },
        Icons::IShapes(ishape) => {
            match avb.mouse.get_mouse_state() {
                LeftDown(mouse_pos_down) => {
                    pointer.set_pos(mouse_pos_down);
                    pointer.save_pos();
                    avb.pools.magnet_to_helpers(&mut pointer, keys_states);

                    if let Some(mut shape) = avb.on_creation.get_shape_into() {
                        // When a custom shape is created, we always continue, we stop (and
                        // close the shape) only when the user clicks on the right button
                        match shape.get_kind_mut() {
                            BSKind::Custom(shape_custom)
                                if matches!(
                                    shape_custom.get_primitivess_start_property(),
                                    VertexProperty::Nope
                                ) =>
                            {
                                // Both conditions succeeded
                                shape_custom.add_point(&mut pointer);
                                avb.on_creation.set_shape(shape);
                            }
                            _ => {
                                // A. We were drawing a new shape
                                if shape.get_kind().good_size() {
                                    if let BSKind::Custom(shape_custom) = shape.get_kind_mut() {
                                        if let VertexProperty::RectangleLike =
                                            shape_custom.get_primitivess_start_property()
                                        {
                                            shape_custom.end_creation();
                                        }
                                    }
                                    // Deselect all
                                    shape.get_kind_mut().set_state(SetSelect(false));
                                    shape.get_kind_mut().set_state(SelectAllModifiers(false));

                                    avb.pools.add_shape(shape.clone());

                                    // Push the AddShapeAction to the undo/redo system
                                    avb.undo_redo.push(Box::new(AddShapeAction {
                                        shape: shape.clone(),
                                    }));

                                    avb.on_creation = DrawObjects::Nope;
                                    avb.pools.recalc_full_segs();
                                }
                            }
                        }
                    } else {
                        // B. We start drawing a new shape
                        avb.on_creation.set_shape(ShapesPool::new_shape(
                            ishape,
                            &mut pointer,
                            BoolOps::Union,
                        ));
                    }
                }

                LeftDownMove(mouse_pos_down, mouse_pos) => (),
                LeftUp(mouse_pos_up) => (),

                LeftUpMove(mouse_pos_up, mouse_pos) | RightUpMove(mouse_pos_up, mouse_pos) => {
                    pointer.set_pos_rel(mouse_pos - mouse_pos_up);
                    avb.pools.magnet_to_helpers(&mut pointer, keys_states);

                    if let Some(shape) = avb.on_creation.get_shape_mut() {
                        _ = shape
                            .get_kind_mut()
                            .move_modifier(&mut pointer, keys_states);
                    }
                }

                RightDown(mouse_pos_down) => {
                    pointer.set_pos(mouse_pos_down);
                    pointer.save_pos();

                    if let Some(mut shape) = avb.on_creation.get_shape_into() {
                        match shape.get_kind_mut() {
                            BSKind::Custom(shape_custom)
                                if matches!(
                                    shape_custom.get_primitivess_start_property(),
                                    VertexProperty::Nope
                                ) =>
                            {
                                // Go out of creation mode
                                if shape_custom.end_creation() {
                                    // Deselect all
                                    shape.get_kind_mut().set_state(SetSelect(false));
                                    shape.get_kind_mut().set_state(SelectAllModifiers(false));
                                    avb.pools.add_shape(shape.clone());

                                    // Push the AddShapeAction to the undo/redo system
                                    avb.undo_redo.push(Box::new(AddShapeAction {
                                        shape: shape.clone(),
                                    }));
                                    avb.on_creation = DrawObjects::Nope;
                                    avb.pools.recalc_full_segs();
                                }
                            }
                            _ => {
                                //
                                avb.on_creation = DrawObjects::Nope;
                                avb.pools.recalc_full_segs();
                            }
                        }
                    }
                    avb.on_creation = DrawObjects::Nope;
                    avb.go_to_arrow_tool();
                }
                _ => (),
            }
        }
        Icons::IHelpers(ihelper) => match avb.mouse.get_mouse_state() {
            LeftDown(mouse_pos_down) => {
                pointer.set_pos(snap_pt(mouse_pos_down, pointer.get_snap().val()));
                pointer.save_pos();
                avb.pools.magnet_to_helpers(&mut pointer, keys_states);

                if let Some(mut helper) = avb.on_creation.get_helper_into() {
                    // Minimum size was not reached
                    if !helper.get_kind().good_size() {
                        log!("Helper too small");
                    } else {
                        // A. We were drawing a new helper, finish all
                        helper.get_kind_mut().set_state(SetSelect(false));
                        helper.get_kind_mut().set_state(SelectAllModifiers(false));

                        avb.pools.add_helper(helper.clone());
                        // Push the AddHelperAction to the undo/redo system
                        avb.undo_redo.push(Box::new(AddHelperAction {
                            helper: helper.clone(),
                        }));
                        // Create a set of magnetic points that are intersections
                        // between the helpers (lines and circles)
                        avb.pools.create_magnet_points();

                        //
                        avb.on_creation = DrawObjects::Nope;
                        avb.pools.recalc_full_segs();
                    }
                } else {
                    // B. We start drawing a new helper
                    avb.on_creation.set_helper(HelpersPool::new_helper(
                        ihelper,
                        pointer.pos(),
                        pointer.pos(),
                    ));
                }
            }
            LeftDownMove(mouse_pos_down, mouse_pos) => {
                pointer.set_pos(snap_pt(mouse_pos, pointer.get_snap().val()));
                avb.pools.magnet_to_helpers(&mut pointer, keys_states);
            }
            LeftUp(mouse_pos_up) => {
                pointer.set_pos(mouse_pos_up);
                pointer.save_pos();
                avb.pools.magnet_to_helpers(&mut pointer, keys_states);
            }
            LeftUpMove(mouse_pos_up, mouse_pos) | RightUpMove(mouse_pos_up, mouse_pos) => {
                pointer.set_pos(snap_pt(mouse_pos, pointer.get_snap().val()));
                avb.pools.magnet_to_helpers(&mut pointer, keys_states);

                if let Some(helper) = avb.on_creation.get_helper_mut() {
                    if helper.get_kind().get_state(IsAnyModifierSelected).is_some() {
                        _ = helper
                            .get_kind_mut()
                            .move_modifier(&mut pointer, keys_states);
                    }
                }
            }
            RightDown(mouse_pos_down) => {
                pointer.set_pos(snap_pt(mouse_pos_down, pointer.get_snap().val()));
                pointer.save_pos();

                avb.on_creation = DrawObjects::Nope;
                avb.go_to_arrow_tool();
            }
            _ => (),
        },
        _ => (),
    }

    avb.pointer = pointer;
    Ok(())
}

fn update_informations(avb: &mut RefMut<'_, AppVars>) {
    avb.canvases.clear_background_canvas();
    //Display: update mouse world position
    let c_size = avb.canvases.get_canvas_size();
    let pos = avb.pointer.pos();
    let ckind = CanvasKind::Background;
    avb.canvases.direct_text(
        &ckind,
        &CanvasText::new(
            format!("( {:.1} , {:.1} )", pos.x, pos.y),
            TextPos::PosCustom(Vec2::new(c_size.width - 10., c_size.height - 10.)),
            CanvasTextConfig::new(Pattern::Rules, 0., TextAlign::Right, 16, 0.4),
        ),
    );
    avb.canvases.direct_text(
        &ckind,
        &CanvasText::new(
            format!(
                "Snap value (PRESS S): {:.0} mm or {:.0} °",
                avb.pointer.get_snap().val(),
                avb.pointer.get_snap().val()
            ),
            TextPos::Pos1(c_size.height),
            CanvasTextConfig::new(Pattern::Rules, 0., TextAlign::Left, 16, 0.4),
        ),
    );
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
    update_informations(&mut pam);
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
    update_informations(&mut pam);
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
    log!("resize called");
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
        if let Some(key) = Keys::from_str(&keyboard_event.key()).ok() {
            let mut avb = av.borrow_mut();
            use Keys::*;
            match key {
                Control | Meta => {
                    log!("control pressed");
                    avb.keys_states.crtl_cmd_pressed = true;
                }
                Shift => {
                    log!("shift pressed");
                    avb.keys_states.shift_pressed = true;
                }
                Alt => {
                    log!("alt pressed");
                    avb.keys_states.alt_pressed = true;
                }
                Delete | Backspace => avb.delete_entity(),
                Escape => avb.cancel_entity_creation(),
                // Copy and paste
                CLower => {
                    if avb.keys_states.crtl_cmd_pressed {
                        log!("ctrl-c pressed");
                        avb.copy_entity();
                    }
                }
                VLower => {
                    if avb.keys_states.crtl_cmd_pressed {
                        log!("ctrl-v pressed");
                        avb.paste_entity();
                    }
                }
                // Undo and Redo
                ZLower => {
                    if avb.keys_states.crtl_cmd_pressed {
                        log!("ctrl-z pressed");
                        avb.undo();
                    }
                }
                ZUpper | YLower => {
                    if avb.keys_states.crtl_cmd_pressed {
                        log!("ctrl-Z pressed or ctrl-y pressed");
                        avb.redo();
                    }
                }
                // Entities values snapping
                SLower | SUpper => {
                    avb.next_snap();
                    update_informations(&mut avb);
                }
                // Toggle boolean operation (add, substract, intersect)
                TLower => avb.toogle_boolean_op(),
                // Change ShapeCustom: A) edge (line, arc,...) or B) vertex (none, fillet, chamfer)
                Tab => {
                    if let Some(shid_custom) = avb.pools.shapes.get_shape_custom_on_select() {
                        avb.change_shape_custom(shid_custom);
                    }
                }
                // Toggle primitive property (concavity for arc)
                Space => {
                    log!("space pressed");
                    avb.toogle_primitive_prop();
                }
                _ => (),
            };

            render_drawing(&mut avb);
            drop(avb);
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
                    avb.keys_states.crtl_cmd_pressed = false;
                }
                Shift => {
                    log!("shift released");
                    avb.keys_states.shift_pressed = false;
                }
                Alt => {
                    log!("alt released");
                    avb.keys_states.alt_pressed = false;
                }
                _ => (),
            }
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
                    avb.pools.shapes.set_state(false, HS::Select);
                    avb.pools.shapes.set_modifiers_state(false, HS::Select);
                    // avb.pool.set_hors_centers(false, HighLightOrSelect::Select);

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
// Rendering
fn reset_origin(avb: &mut RefMut<'_, AppVars>) {
    avb.canvases.reset_origin();
}
fn draw_grid_and_rules(avb: &mut RefMut<'_, AppVars>) {
    avb.canvases.draw_origin();
}
fn render_drawing(avb: &mut RefMut<'_, AppVars>) {
    // Get the Performance API
    // let performance = window().unwrap().performance().unwrap();
    // let start_time = performance.now();

    let _scale = avb.canvases.get_drawing_scale();
    avb.canvases.clear_main_canvas();
    let das = &avb.canvases.get_drawing_size();
    let cinfo = avb.canvases.get_canvas_infos();

    // Draw pointer
    if avb.pointer.is_active() {
        avb.canvases.draw_pointer(avb.pointer.pos());
    }

    // Draw the final contour shapes
    let full_segs = avb.pools.shapes.get_full_segs();
    avb.canvases.draw_closed_path(
        &CanvasKind::Draw,
        full_segs,
        Pattern::ComposedNormal(true),
        vec![],
    );

    // SHAPES: Draw the outline of every shape
    // log!("START");
    for shape in avb.pools.shapes.values() {
        shape
            .get_kind()
            .get_paths_and_patterns(das, cinfo)
            .iter()
            .for_each(|(path, pattern)| {
                avb.canvases
                    .draw_path(&CanvasKind::Draw, (path.clone(), *pattern), vec![]);
            });
    }

    // SHAPES: Draw the modifiers points
    for shape in avb.pools.shapes.values() {
        for path in shape.get_kind().get_mod_paths_and_patterns(das, cinfo) {
            avb.canvases.draw_path(&CanvasKind::Draw, path, vec![]);
        }
    }

    // SHAPES: Draw dimensions
    use GetEntityState::*;
    for shape in avb.pools.shapes.values() {
        if shape.get_kind().get_state(IsSelected).is_some()
            || shape.get_kind().get_state(IsHighligh).is_some()
            || shape.get_kind().get_state(IsAnyModifierSelected).is_some()
            || shape.get_kind().get_state(IsAnyModifierHighligh).is_some()
        {
            for (path, pattern, text) in shape
                .get_kind()
                .get_dimensions_paths_and_patterns(das, cinfo)
            {
                avb.canvases
                    .draw_path(&CanvasKind::Draw, (path, pattern), vec![text]);
            }
        }
    }

    // HELPERS: Draw the helpers
    for helper in avb.pools.helpers.values() {
        helper
            .get_kind()
            .get_paths_and_patterns(das, cinfo)
            .iter()
            .for_each(|(path, pattern)| {
                avb.canvases
                    .draw_path(&CanvasKind::Draw, (path.clone(), *pattern), vec![]);
            });
    }
    // HELPERS: Draw the modifiers points
    for helper in avb.pools.helpers.values() {
        for path in helper.get_kind().get_mod_paths_and_patterns(das, cinfo) {
            avb.canvases.draw_path(&CanvasKind::Draw, path, vec![]);
        }
    }

    // HELPERS: Draw dimensions
    for helper in avb.pools.helpers.values() {
        if helper.get_kind().get_state(IsSelected).is_some()
            || helper.get_kind().get_state(IsHighligh).is_some()
            || helper.get_kind().get_state(IsAnyModifierSelected).is_some()
            || helper.get_kind().get_state(IsAnyModifierHighligh).is_some()
        {
            for (path, pattern, text) in helper
                .get_kind()
                .get_dimensions_paths_and_patterns(das, cinfo)
            {
                avb.canvases
                    .draw_path(&CanvasKind::Draw, (path, pattern), vec![text]);
            }
        }
    }
    // HELPERS: Draw the magnets points
    for magnet_point in avb.pools.helpers.magnet_points() {
        avb.canvases.draw_path(
            &CanvasKind::Draw,
            (modifiers_path(*magnet_point, 1., 5.), Pattern::HelperNormal),
            vec![],
        );
    }

    // Draw the clipboard item if any
    if let Some(item) = avb.clipboard.get_paste() {
        match item {
            ClipboardItem::Shapes((shapes, _)) => {
                for shape in shapes {
                    shape
                        .get_kind()
                        .get_paths_and_patterns(das, cinfo)
                        .iter()
                        .for_each(|(path, pattern)| {
                            avb.canvases.draw_path(
                                &CanvasKind::Draw,
                                (path.clone(), *pattern),
                                vec![],
                            );
                        });
                }
            }
            ClipboardItem::Helpers((helpers, _)) => {
                for helper in helpers {
                    helper
                        .get_kind()
                        .get_paths_and_patterns(das, cinfo)
                        .iter()
                        .for_each(|(path, pattern)| {
                            avb.canvases.draw_path(
                                &CanvasKind::Draw,
                                (path.clone(), *pattern),
                                vec![],
                            );
                        });
                }
            }
        }
    }

    // Draw the on_creation object if any
    if let Some(shape) = avb.on_creation.get_shape() {
        shape
            .get_kind()
            .get_paths_and_patterns(das, cinfo)
            .iter()
            .for_each(|(path, pattern)| {
                avb.canvases
                    .draw_path(&CanvasKind::Draw, (path.clone(), *pattern), vec![]);
            });
        // With modifiers
        for path in shape.get_kind().get_mod_paths_and_patterns(das, cinfo) {
            avb.canvases.draw_path(&CanvasKind::Draw, path, vec![]);
        }
        // With dimensions
        for (path, pattern, text) in shape
            .get_kind()
            .get_dimensions_paths_and_patterns(das, cinfo)
        {
            avb.canvases
                .draw_path(&CanvasKind::Draw, (path, pattern), vec![text]);
        }
    } else {
        if let Some(helper) = avb.on_creation.get_helper() {
            helper
                .get_kind()
                .get_paths_and_patterns(das, cinfo)
                .iter()
                .for_each(|(path, pattern)| {
                    avb.canvases
                        .draw_path(&CanvasKind::Draw, (path.clone(), *pattern), vec![]);
                });
            // With modifiers
            for path in helper.get_kind().get_mod_paths_and_patterns(das, cinfo) {
                avb.canvases.draw_path(&CanvasKind::Draw, path, vec![]);
            }
            // With dimensions
            for (path, pattern, text) in helper
                .get_kind()
                .get_dimensions_paths_and_patterns(das, cinfo)
            {
                avb.canvases
                    .draw_path(&CanvasKind::Draw, (path, pattern), vec![text]);
            }
        }
    }

    // let end_time = performance.now();
    // log!("Rendering time: {:.2} ms", end_time - start_time);
}
