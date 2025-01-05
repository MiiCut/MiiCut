use std::vec;

use js_sys::Array;
use kurbo::{BezPath, PathEl, Point, Size, Vec2};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, CssStyleDeclaration, HtmlCanvasElement, Window};

use crate::math::*;

// #![cfg(not(test))]
// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Align {
    Left,
    Right,
    Center,
}
#[derive(Clone, Debug)]
pub struct CanvasText {
    pub text: String,
    pub pos: Vec2,
    pub pattern: Pattern,
    pub angle: f64,
    pub align: Align,
    pub font_size: u32,
    pub opacity: f64,
}

#[derive(Clone, Debug)]
pub enum CanvasKind {
    Background,
    Grid,
    Draw,
}

#[derive(Clone, Debug)]
struct GridRules {
    h_rule_height: f64,
    v_rule_width: f64,
    primary_rules_thicks_hw: f64,
    secondary_rules_thicks_hw: f64,
}
impl GridRules {
    fn new() -> GridRules {
        GridRules {
            h_rule_height: 0.,
            v_rule_width: 0.,
            primary_rules_thicks_hw: 8.,
            secondary_rules_thicks_hw: 4.,
        }
    }
    pub fn draw_rules(
        &mut self,
        draw_rec_size: Size,
        draw_rec_grid_spacing: f64,
        drawing_offset: Vec2,
        drawing_scale: f64,
    ) -> (CanvasKind, Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        use PathEl::*;
        let mut v: Vec<PathEl> = vec![];
        let mut texts: Vec<CanvasText> = vec![];

        let d = to_draw(Vec2::ZERO, drawing_scale, drawing_offset);
        // Horizontal rule
        let offset_y = if d.y < -self.h_rule_height {
            0.
        } else {
            d.y + self.h_rule_height
        };
        let mut wx = 0.;
        while wx <= draw_rec_size.width {
            let h = if (wx / (10. * draw_rec_grid_spacing)).fract() == 0. {
                texts.push(CanvasText {
                    text: format!("{:.0}", wx / 10.),
                    pos: Vec2::new(wx, offset_y + 15.),
                    pattern: Pattern::Rules,
                    angle: 0.,
                    align: Align::Center,
                    font_size: 16,
                    opacity: 0.4,
                });
                self.primary_rules_thicks_hw
            } else {
                self.secondary_rules_thicks_hw
            };
            v.push(MoveTo(Point::new(wx, offset_y)));
            v.push(LineTo(Point::new(wx, offset_y + h)));
            wx += draw_rec_grid_spacing
        }

        // Vertical rule
        let offset_x = if d.x < -self.v_rule_width {
            0.
        } else {
            d.x + self.v_rule_width
        };
        let mut wy = 0.;
        while wy <= draw_rec_size.height {
            let w = if (wy / (10. * draw_rec_grid_spacing)).fract() == 0. {
                texts.push(CanvasText {
                    text: format!("{:.0}", wy / 10.),
                    pos: Vec2::new(offset_x + 5., wy),
                    pattern: Pattern::Rules,
                    angle: 0.,
                    align: Align::Left,
                    font_size: 16,
                    opacity: 0.4,
                });
                self.primary_rules_thicks_hw
            } else {
                self.secondary_rules_thicks_hw
            };
            v.push(MoveTo(Point::new(offset_x, wy)));
            v.push(LineTo(Point::new(offset_x + w, wy)));
            wy += draw_rec_grid_spacing;
        }

        (
            CanvasKind::Grid,
            vec![(BezPath::from_vec(v), Pattern::Rules)],
            texts,
        )
    }

    pub fn draw_grid_primary(
        &mut self,
        draw_rec_size: Size,
        draw_rec_grid_spacing: f64,
    ) -> (CanvasKind, Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        use PathEl::*;
        let mut v: Vec<PathEl> = vec![];
        let spacing = 10. * draw_rec_grid_spacing;
        // Vertical grid lines
        let mut wx = 0.;

        while wx <= draw_rec_size.width {
            if (wx / spacing).fract() == 0. {
                v.push(MoveTo(Point::new(wx, 0.)));
                v.push(LineTo(Point::new(wx, draw_rec_size.height)));
            }
            wx += draw_rec_grid_spacing
        }
        // Horizontal grid lines
        let mut wy = 0.;
        while wy <= draw_rec_size.height {
            if (wy / spacing).fract() == 0. {
                v.push(MoveTo(Point::new(0., wy)));
                v.push(LineTo(Point::new(draw_rec_size.width, wy)));
            }
            wy += draw_rec_grid_spacing;
        }

        (
            CanvasKind::Grid,
            vec![(BezPath::from_vec(v), Pattern::GridPrimary)],
            vec![],
        )
    }

    pub fn draw_grid_secondary(
        &mut self,
        draw_rec_size: Size,
        draw_rec_grid_spacing: f64,
    ) -> (CanvasKind, Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        use PathEl::*;
        let mut v: Vec<PathEl> = vec![];

        // Vertical grid lines
        let mut wx = 0.;

        while wx <= draw_rec_size.width {
            v.push(MoveTo(Point::new(wx, 0.)));
            v.push(LineTo(Point::new(wx, draw_rec_size.height)));

            wx += draw_rec_grid_spacing
        }
        // Horizontal grid lines
        let mut wy = 0.;
        while wy <= draw_rec_size.height {
            v.push(MoveTo(Point::new(0., wy)));
            v.push(LineTo(Point::new(draw_rec_size.width, wy)));

            wy += draw_rec_grid_spacing;
        }

        (
            CanvasKind::Grid,
            vec![(BezPath::from_vec(v), Pattern::GridSecondary)],
            vec![],
        )
    }
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

    grid_rules: GridRules,
    grid_size: f64,
    grid_snap: f64,

    styles: DrawStyles,
}
impl Canvases {
    pub fn new(
        window: Window,
        c_back: HtmlCanvasElement,
        c_grid: HtmlCanvasElement,
        c_main: HtmlCanvasElement,
        drawing_size: Size,
    ) -> Result<Canvases, JsValue> {
        let document = window.document().expect("should have a document on window");

        let document_element = document
            .document_element()
            .ok_or("should have a document element")?;
        let css_styles = window
            .get_computed_style(&document_element)
            .unwrap()
            .unwrap();
        let styles = DrawStyles::build(css_styles)?;
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
        Ok(Canvases {
            c_back,
            c_grid,
            c_main,
            c_back_ctx,
            c_grid_ctx,
            c_main_ctx,
            drawing_area_size: drawing_size,
            drawing_offset_saved: Vec2::new(50., 50.),
            drawing_offset: Vec2::new(50., 50.),
            drawing_scale: 2.,
            grid_rules: GridRules::new(),
            grid_size: 10.,
            grid_snap: 1.,
            styles,
        })
    }
    pub fn clear_background_canvas(&mut self) {
        self.c_back_ctx.clear_rect(
            0.,
            0.,
            self.c_main.width() as f64,
            self.c_main.height() as f64,
        );
    }

    pub fn draw_grid_and_rules(&mut self) {
        self.c_grid_ctx.clear_rect(
            0.,
            0.,
            self.c_main.width() as f64,
            self.c_main.height() as f64,
        );

        // Rules
        let (canvas_kind, bez_path, texts) = self.grid_rules.draw_rules(
            self.get_drawing_size(),
            self.get_grid_size(),
            self.drawing_offset,
            self.drawing_scale,
        );
        self.draw_path(&canvas_kind, bez_path, texts);

        // Primary grid
        let (canvas_kind, bez_path, texts) = self
            .grid_rules
            .draw_grid_primary(self.get_drawing_size(), self.get_grid_size());
        self.draw_path(&canvas_kind, bez_path, texts);

        // Secondary grid
        let (canvas_kind, bez_path, texts) = self
            .grid_rules
            .draw_grid_secondary(self.get_drawing_size(), self.get_grid_size());
        self.draw_path(&canvas_kind, bez_path, texts);
    }

    pub fn clear_main_canvas(&mut self) {
        self.c_main_ctx.clear_rect(
            0.,
            0.,
            self.c_main.width() as f64,
            self.c_main.height() as f64,
        );
    }

    pub fn draw_text(&self, canvas_kind: &CanvasKind, text: &CanvasText) {
        let ctx = self.get_context(canvas_kind);
        let scale = self.get_drawing_scale();
        let offset = self.get_drawing_offset();

        ctx.save();
        let cpt = to_canvas(text.pos, scale, offset);

        ctx.translate(cpt.x, cpt.y)
            .expect("Failed to translate canvas");

        ctx.rotate(text.angle).expect("Failed to rotate canvas");
        ctx.set_font("14px Orbitron");
        ctx.set_font(&format!("{}px Orbitron", text.font_size));
        ctx.set_global_alpha(text.opacity);
        let (stroke_style, stroke_width, _) = self.styles.get_styles(text.pattern);
        ctx.set_line_dash(stroke_style).unwrap();
        ctx.set_line_width(stroke_width);
        let (fill_color, stroke_color) = self.styles.get_colors(text.pattern);
        ctx.set_stroke_style_str(stroke_color);
        ctx.set_fill_style_str(fill_color);
        ctx.set_text_align(match text.align {
            Align::Left => "left",
            Align::Right => "right",
            Align::Center => "center",
        });
        ctx.fill_text(&text.text, 0., 0.)
            .expect("Failed to draw text");
        ctx.restore();
    }

    pub fn draw_path(
        &self,
        canvas_kind: &CanvasKind,
        paths: Vec<(BezPath, Pattern)>,
        texts: Vec<CanvasText>,
    ) {
        let ctx = self.get_context(&canvas_kind);
        let scale = self.get_drawing_scale();
        let offset = self.get_drawing_offset();
        ctx.set_font("14px Orbitron");

        for (path, pattern) in paths.iter() {
            let (stroke_style, stroke_width, filled) = self.styles.get_styles(*pattern);
            ctx.set_line_dash(stroke_style).unwrap();
            ctx.set_line_width(stroke_width);
            let (fill_color, stroke_color) = self.styles.get_colors(*pattern);
            ctx.set_stroke_style_str(stroke_color);
            ctx.set_fill_style_str(fill_color);
            ctx.begin_path();
            for cst in path.iter() {
                match cst {
                    PathEl::MoveTo(pt) => {
                        let cpt = to_canvas(pt.to_vec2(), scale, offset);
                        ctx.move_to(cpt.x, cpt.y);
                    }
                    PathEl::LineTo(pt) => {
                        let cpt = to_canvas(pt.to_vec2(), scale, offset);
                        ctx.line_to(cpt.x, cpt.y);
                    }
                    PathEl::QuadTo(pt1, pt2) => {
                        let cpt1 = to_canvas(pt1.to_vec2(), scale, offset);
                        let cpt2 = to_canvas(pt2.to_vec2(), scale, offset);
                        ctx.quadratic_curve_to(cpt1.x, cpt1.y, cpt2.x, cpt2.y);
                    }
                    PathEl::CurveTo(pt1, pt2, pt3) => {
                        let cpt1 = to_canvas(pt1.to_vec2(), scale, offset);
                        let cpt2 = to_canvas(pt2.to_vec2(), scale, offset);
                        let cpt3 = to_canvas(pt3.to_vec2(), scale, offset);
                        ctx.bezier_curve_to(cpt1.x, cpt1.y, cpt2.x, cpt2.y, cpt3.x, cpt3.y);
                    }
                    PathEl::ClosePath => ctx.close_path(),
                }
            }
            if filled {
                ctx.fill();
            }
            ctx.stroke();
        }
        // Drawing the texts
        for text in texts.iter() {
            self.draw_text(canvas_kind, text);
        }
    }
    pub fn draw_closed_path(
        &self,
        canvas_kind: &CanvasKind,
        paths: Vec<BezPath>,
        pattern: Pattern,
        texts: Vec<CanvasText>,
    ) {
        let ctx = self.get_context(&canvas_kind);
        let scale = self.get_drawing_scale();
        let offset = self.get_drawing_offset();

        let (stroke_style, stroke_width, filled) = self.styles.get_styles(pattern);
        ctx.set_line_dash(stroke_style).unwrap();
        ctx.set_line_width(stroke_width);
        let (fill_color, stroke_color) = self.styles.get_colors(pattern);
        ctx.set_stroke_style_str(stroke_color);
        ctx.set_fill_style_str(fill_color);
        ctx.set_font("14px Orbitron");
        ctx.begin_path();
        for (_idx, path) in paths.iter().enumerate() {
            for cst in path.iter() {
                match cst {
                    PathEl::MoveTo(pt) => {
                        let cpt = to_canvas(pt.to_vec2(), scale, offset);
                        ctx.move_to(cpt.x, cpt.y);
                    }
                    PathEl::LineTo(pt) => {
                        let cpt = to_canvas(pt.to_vec2(), scale, offset);
                        ctx.line_to(cpt.x, cpt.y);
                    }
                    PathEl::QuadTo(pt1, pt2) => {
                        let cpt1 = to_canvas(pt1.to_vec2(), scale, offset);
                        let cpt2 = to_canvas(pt2.to_vec2(), scale, offset);
                        ctx.quadratic_curve_to(cpt1.x, cpt1.y, cpt2.x, cpt2.y);
                    }
                    PathEl::CurveTo(pt1, pt2, pt3) => {
                        let cpt1 = to_canvas(pt1.to_vec2(), scale, offset);
                        let cpt2 = to_canvas(pt2.to_vec2(), scale, offset);
                        let cpt3 = to_canvas(pt3.to_vec2(), scale, offset);
                        ctx.bezier_curve_to(cpt1.x, cpt1.y, cpt2.x, cpt2.y, cpt3.x, cpt3.y);
                    }
                    PathEl::ClosePath => ctx.close_path(),
                }
            }
        }
        if filled {
            ctx.fill();
        }
        ctx.stroke();
        // Drawing the texts
        for text in texts.iter() {
            self.draw_text(canvas_kind, text);
        }
    }

    pub fn draw_pointer(&self, position: Vec2) {
        let ctx = self.get_context(&CanvasKind::Draw);
        let scale = self.get_drawing_scale();
        let offset = self.get_drawing_offset();
        let canvas_size = self.get_canvas_size();
        let pos_canvas = to_canvas(position, scale, offset);

        let (stroke_style, stroke_width, _) = self.styles.get_styles(Pattern::Rules);
        ctx.set_line_dash(stroke_style).unwrap();
        ctx.set_line_width(stroke_width);
        let (fill_color, stroke_color) = self.styles.get_colors(Pattern::Rules);
        ctx.set_stroke_style_str(stroke_color);
        ctx.set_fill_style_str(fill_color);

        ctx.set_font("14px Orbitron");
        ctx.begin_path();

        ctx.move_to(0., pos_canvas.y);
        ctx.line_to(canvas_size.width, pos_canvas.y);
        ctx.move_to(pos_canvas.x, 0.);
        ctx.line_to(pos_canvas.x, canvas_size.height);
        ctx.close_path();

        ctx.stroke();
    }

    pub fn get_main_canvas(&self) -> &HtmlCanvasElement {
        &self.c_main
    }
    pub fn get_context(&self, kind: &CanvasKind) -> &CanvasRenderingContext2d {
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
            cursor_pos - pos_dwn,
            self.drawing_scale,
            self.drawing_offset,
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
#[derive(Debug, Clone)]
pub struct DrawStyles {
    transparent_color: String,
    background_color: String,
    grid_primary_color: String,
    grid_secondary_color: String,
    rules_color: String,
    main_color: String,
    modifiers_color: String,
    modifiers_highlight_color: String,
    modifiers_selected_color: String,
    // Drawing colors
    basic_normal_color: String,
    basic_highlight_color: String,
    basic_selected_color: String,

    composed_normal_color: String,
    composed_highlight_color: String,
    composed_selected_color: String,

    composed_normal_fill_color: String,
    composed_highlight_fill_color: String,
    composed_selected_fill_color: String,

    dimension_normal_color: String,
    dimension_highlight_color: String,
    dimension_selected_color: String,

    // line patterns
    pattern_dashed: JsValue,
    pattern_solid: JsValue,
}
impl DrawStyles {
    pub fn build(style: CssStyleDeclaration) -> Result<DrawStyles, JsValue> {
        let background_color = style.get_property_value("--canvas-background-color")?;
        let grid_primary_color = style.get_property_value("--canvas-grid-primary-color")?;
        let grid_secondary_color = style.get_property_value("--canvas-grid-secondary-color")?;
        let rules_color = style.get_property_value("--canvas-rules-color")?;
        let main_color = style.get_property_value("--canvas-main-color")?;
        let modifiers_color = style.get_property_value("--canvas-modifiers-color")?;
        let modifiers_highlight_color =
            style.get_property_value("--canvas-modifiers-highlight-color")?;
        let modifiers_selected_color =
            style.get_property_value("--canvas-modifiers-selected-color")?;
        let basic_normal_color = style.get_property_value("--canvas-basic-normal-color")?;
        let basic_highlight_color = style.get_property_value("--canvas-basic-highlight-color")?;
        let basic_selected_color = style.get_property_value("--canvas-basic-selected-color")?;
        let composed_normal_color = style.get_property_value("--canvas-composed-normal-color")?;
        let composed_highlight_color =
            style.get_property_value("--canvas-composed-highlight-color")?;
        let composed_selected_color =
            style.get_property_value("--canvas-composed-selected-color")?;
        let composed_normal_fill_color =
            style.get_property_value("--canvas-composed-normal-fill-color")?;
        let composed_highlight_fill_color =
            style.get_property_value("--canvas-composed-highlight-fill-color")?;
        let composed_selected_fill_color =
            style.get_property_value("--canvas-composed-selected-fill-color")?;
        let dimension_normal_color = style.get_property_value("--canvas-dimension-normal-color")?;
        let dimension_highlight_color =
            style.get_property_value("--canvas-dimension-highlight-color")?;
        let dimension_selected_color =
            style.get_property_value("--canvas-dimension-selected-color")?;

        let transparent_color = style.get_property_value("--canvas-transparent-color")?;

        let dash_pattern = Array::new();
        dash_pattern.push(&JsValue::from_f64(4.0));
        dash_pattern.push(&JsValue::from_f64(10.0));
        let solid_pattern = Array::new();
        Ok(DrawStyles {
            background_color,
            grid_primary_color,
            grid_secondary_color,
            rules_color,
            main_color,
            modifiers_color,
            modifiers_highlight_color,
            modifiers_selected_color,
            //
            basic_normal_color,
            basic_highlight_color,
            basic_selected_color,
            composed_normal_color,
            composed_highlight_color,
            composed_selected_color,
            composed_normal_fill_color,
            composed_highlight_fill_color,
            composed_selected_fill_color,
            dimension_normal_color,
            dimension_highlight_color,
            dimension_selected_color,
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
            GridPrimary => (&self.pattern_solid, 1., false),
            GridSecondary => (&self.pattern_solid, 1., false),
            Rules => (&self.pattern_solid, 1., false),
            Modifiers => (&self.pattern_solid, 1., true),
            ModifiersHighlighted => (&self.pattern_solid, 1., true),
            ModifiersSelected => (&self.pattern_solid, 1., true),
            ComposedNormal(filled) => (&self.pattern_solid, 3., filled),
            ComposedHighlighted(filled) => (&self.pattern_solid, 3., filled),
            ComposedSelected(filled) => (&self.pattern_solid, 3., filled),
            BasicNormal => (&self.pattern_dashed, 1., false),
            BasicHighlighted => (&self.pattern_dashed, 1., false),
            BasicSelected => (&self.pattern_solid, 1., false),
            DimensionNormal => (&self.pattern_solid, 1., false),
            DimensionHighlighted => (&self.pattern_solid, 1., false),
            DimensionSelected => (&self.pattern_solid, 1., false),
        };
        (line_dash, line_width, filled)
    }
    pub fn get_colors(&self, pattern: Pattern) -> (&str, &str) {
        use Pattern::*;
        let (fill_color, color) = match pattern {
            GridPrimary => (&self.grid_primary_color, &self.grid_primary_color),
            GridSecondary => (&self.grid_secondary_color, &self.grid_secondary_color),
            Rules => (&self.rules_color, &self.rules_color),
            Modifiers => (&self.modifiers_color, &self.modifiers_color),
            ModifiersHighlighted => (
                &self.modifiers_highlight_color,
                &self.modifiers_highlight_color,
            ),
            ModifiersSelected => (
                &self.modifiers_selected_color,
                &self.modifiers_selected_color,
            ),
            ComposedNormal(_) => (
                &self.composed_normal_fill_color,
                &self.composed_normal_color,
            ),
            ComposedHighlighted(_) => (
                &self.composed_highlight_fill_color,
                &self.composed_highlight_color,
            ),
            ComposedSelected(_) => (
                &self.composed_selected_fill_color,
                &self.composed_selected_color,
            ),
            BasicNormal => (&self.transparent_color, &self.basic_normal_color),
            BasicHighlighted => (&self.transparent_color, &self.basic_highlight_color),
            BasicSelected => (&self.basic_selected_color, &self.basic_selected_color),

            DimensionNormal => (&self.dimension_normal_color, &self.dimension_normal_color),
            DimensionHighlighted => (
                &self.dimension_highlight_color,
                &self.dimension_highlight_color,
            ),
            DimensionSelected => (
                &self.dimension_selected_color,
                &self.dimension_selected_color,
            ),
        };
        (fill_color, color)
    }
    pub fn get_background_color(&self) -> &str {
        &self.background_color
    }
    pub fn get_transparent_color(&self) -> &str {
        &self.transparent_color
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Pattern {
    GridPrimary,
    GridSecondary,
    Rules,
    Modifiers,
    ModifiersHighlighted,
    ModifiersSelected,
    ComposedNormal(bool),
    ComposedHighlighted(bool),
    ComposedSelected(bool),
    BasicNormal,
    BasicHighlighted,
    BasicSelected,
    DimensionNormal,
    DimensionHighlighted,
    DimensionSelected,
}
