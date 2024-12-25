use js_sys::Array;
use kurbo::{Size, Vec2};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, CssStyleDeclaration, HtmlCanvasElement};

use crate::to_canvas;

// #![cfg(not(test))]
// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
#[derive(Clone, Debug)]
pub enum CanvasKind {
    Background,
    Grid,
    Draw,
}

#[derive(Clone, Debug)]
pub struct Canvases {
    c_back: HtmlCanvasElement,
    c_grid: HtmlCanvasElement,
    c_main: HtmlCanvasElement,
    c_back_ctx: CanvasRenderingContext2d,
    c_grid_ctx: CanvasRenderingContext2d,
    c_main_ctx: CanvasRenderingContext2d,

    // The size (mm,mm) = (px,px) of the drawing area
    drawing_area_size: Size,
    // Offset and scale of the drawing on the canvas
    drawing_offset_saved: Vec2,
    drawing_offset: Vec2,
    drawing_scale: f64,

    grid_size: f64,
    grid_snap: f64,
}
impl Canvases {
    pub fn new(
        c_back: HtmlCanvasElement,
        c_grid: HtmlCanvasElement,
        c_main: HtmlCanvasElement,
        drawing_size: Size,
    ) -> Canvases {
        log!(
            "c_back rect {} {} c_grid rect {} {} c_draw rect {} {}",
            c_back.get_bounding_client_rect().width(),
            c_back.get_bounding_client_rect().height(),
            c_grid.get_bounding_client_rect().width(),
            c_grid.get_bounding_client_rect().height(),
            c_main.get_bounding_client_rect().width(),
            c_main.get_bounding_client_rect().height(),
        );
        let c_back_ctx = c_back
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();
        let c_grid_ctx = c_grid
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();
        let c_main_ctx = c_main
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();
        Canvases {
            c_back,
            c_grid,
            c_main,
            c_back_ctx,
            c_grid_ctx,
            c_main_ctx,
            drawing_area_size: drawing_size,
            drawing_offset_saved: Vec2::new(50., 50.),
            drawing_offset: Vec2::new(50., 50.),
            drawing_scale: 0.5,
            grid_size: 10.,
            grid_snap: 1.,
        }
    }
    pub fn clear_background_canvas(&mut self) {
        self.c_back_ctx.clear_rect(
            0.,
            0.,
            self.c_main.width() as f64,
            self.c_main.height() as f64,
        );
    }
    pub fn clear_grid_canvas(&mut self) {
        self.c_grid_ctx.clear_rect(
            0.,
            0.,
            self.c_main.width() as f64,
            self.c_main.height() as f64,
        );
    }
    pub fn clear_main_canvas(&mut self) {
        self.c_main_ctx.clear_rect(
            0.,
            0.,
            self.c_main.width() as f64,
            self.c_main.height() as f64,
        );
    }
    pub fn get_main_canvas(&self) -> &HtmlCanvasElement {
        &self.c_main
    }
    pub fn get_context(&self, kind: CanvasKind) -> &CanvasRenderingContext2d {
        use CanvasKind::*;
        match kind {
            Background => &self.c_back_ctx,
            Grid => &self.c_grid_ctx,
            Draw => &self.c_main_ctx,
        }
    }
    pub fn get_grid_canvas_context(&self) -> &CanvasRenderingContext2d {
        &self.c_grid_ctx
    }
    pub fn get_main_canvas_context(&self) -> &CanvasRenderingContext2d {
        &self.c_main_ctx
    }
    pub fn get_size(&self) -> Size {
        Size::new(self.c_main.width() as f64, self.c_main.height() as f64)
    }
    pub fn resize_canvases(&mut self, width: u32, height: u32) {
        self.c_back.set_width(width);
        self.c_back.set_height(height);
        self.c_grid.set_width(width);
        self.c_grid.set_height(height);
        self.c_main.set_width(width);
        self.c_main.set_height(height);
        // Update the drawing area
    }
    pub fn get_drawing_scale(&self) -> f64 {
        self.drawing_scale
    }
    pub fn get_drawing_size(&self) -> Size {
        self.drawing_area_size
    }
    pub fn get_drawing_offset(&self) -> Vec2 {
        self.drawing_offset
    }
    pub fn set_drawing_offset(&mut self, offset: Vec2) {
        self.drawing_offset = offset
    }
    pub fn move_drawing_offset(&mut self, pos_dwn: Vec2, cursor_pos: Vec2) {
        self.drawing_offset = to_canvas(
            &(cursor_pos - pos_dwn),
            self.drawing_scale,
            &self.drawing_offset,
        );
    }
    pub fn get_drawing_offset_saved(&mut self) -> Vec2 {
        self.drawing_offset_saved
    }
    pub fn save_drawing_offset(&mut self) {
        self.drawing_offset_saved = self.drawing_offset
    }
    pub fn set_drawing_scale(&mut self, scale: f64) {
        self.drawing_scale = scale
    }
    pub fn set_grid_size(&mut self, size: f64) {
        self.grid_size = size
    }
    pub fn set_grid_snap(&mut self, size: f64) {
        self.grid_snap = size
    }
    pub fn get_grid_size(&self) -> f64 {
        self.grid_size
    }
    pub fn get_grid_snap(&self) -> f64 {
        self.grid_snap
    }
    pub fn get_canvas_offset(&self) -> Vec2 {
        Vec2::new(
            self.c_main.get_bounding_client_rect().x(),
            self.c_main.get_bounding_client_rect().y(),
        )
    }
    pub fn get_canvas_size(&self) -> Size {
        Size::new(
            self.c_main.get_bounding_client_rect().width(),
            self.c_main.get_bounding_client_rect().height(),
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Layer {
    Worksheet,
    Dimension,
    Constraints,
    GeometryHelpers,
    Origin,
    Grid,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DrawStyles {
    background_color: String,
    grid_color: String,
    main_color: String,
    // Drawing colors
    normal_color: String,
    highlight_color: String,
    selected_color: String,
    normal_fill_color: String,
    highlight_fill_color: String,
    selected_fill_color: String,
    //
    transparent_color: String,
    // line patterns
    pattern_dashed: JsValue,
    pattern_solid: JsValue,
}
impl DrawStyles {
    pub fn build(style: CssStyleDeclaration) -> Result<DrawStyles, JsValue> {
        let background_color = style.get_property_value("--canvas-background-color")?;
        let grid_color = style.get_property_value("--canvas-grid-color")?;
        let main_color = style.get_property_value("--canvas-main-color")?;

        let normal_color = style.get_property_value("--canvas-normal-color")?;
        let highlight_color = style.get_property_value("--canvas-highlight-color")?;
        let selected_color = style.get_property_value("--canvas-selected-color")?;
        let normal_fill_color = style.get_property_value("--canvas-normal-fill-color")?;
        let highlight_fill_color = style.get_property_value("--canvas-highlight-fill-color")?;
        let selected_fill_color = style.get_property_value("--canvas-selected-fill-color")?;

        let transparent_color = style.get_property_value("--canvas-transparent-color")?;

        let dash_pattern = Array::new();
        dash_pattern.push(&JsValue::from_f64(10.0));
        dash_pattern.push(&JsValue::from_f64(10.0));
        let solid_pattern = Array::new();
        Ok(DrawStyles {
            background_color,
            grid_color,
            main_color,
            //
            normal_color,
            highlight_color,
            selected_color,
            normal_fill_color,
            highlight_fill_color,
            selected_fill_color,
            //
            transparent_color,
            //
            pattern_dashed: JsValue::from(dash_pattern),
            pattern_solid: JsValue::from(solid_pattern),
        })
    }
    pub fn get_styles(&self, pattern: Pattern) -> (&JsValue, f64, bool) {
        use Pattern::*;
        let (line_dash, line_width, filled) = match pattern {
            Grid => (&self.pattern_solid, 1., false),
            ComposedNormal(filled) => (&self.pattern_solid, 2., filled),
            ComposedHighlighted(filled) => (&self.pattern_solid, 2., filled),
            ComposedSelected(filled) => (&self.pattern_solid, 2., filled),
            BasicNormal => (&self.pattern_dashed, 1., false),
            BasicHighlighted => (&self.pattern_dashed, 1., false),
            BasicSelected => (&self.pattern_dashed, 1., false),
            HandleNormal(filled) => (&self.pattern_solid, 1., filled),
            HandleHighlighted(filled) => (&self.pattern_solid, 1., filled),
            HandleSelected(filled) => (&self.pattern_solid, 1., filled),
        };
        (line_dash, line_width, filled)
    }
    pub fn get_colors(&self, pattern: Pattern) -> (&str, &str) {
        use Pattern::*;
        let (fill_color, color) = match pattern {
            Grid => (&self.grid_color, &self.grid_color),
            ComposedNormal(_) => (&self.normal_fill_color, &self.normal_color),
            ComposedHighlighted(_) => (&self.highlight_fill_color, &self.highlight_color),
            ComposedSelected(_) => (&self.selected_fill_color, &self.selected_color),
            BasicNormal => (&self.transparent_color, &self.normal_color),
            BasicHighlighted => (&self.transparent_color, &self.highlight_color),
            BasicSelected => (&self.transparent_color, &self.selected_color),
            HandleNormal(_) => (&self.normal_fill_color, &self.normal_color),
            HandleHighlighted(_) => (&self.highlight_fill_color, &self.highlight_color),
            HandleSelected(_) => (&self.selected_fill_color, &self.selected_color),
        };
        (fill_color, color)
    }
    pub fn get_background_color(&self) -> &str {
        &self.background_color
    }
    pub fn get_transparent_color(&self) -> &str {
        &self.transparent_color
    }
    pub fn get_selected_color(&self) -> &str {
        &self.selected_color
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Pattern {
    Grid,
    ComposedNormal(bool),
    ComposedHighlighted(bool),
    ComposedSelected(bool),
    BasicNormal,
    BasicHighlighted,
    BasicSelected,
    HandleNormal(bool),
    HandleHighlighted(bool),
    HandleSelected(bool),
}
