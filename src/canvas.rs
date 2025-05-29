// #![cfg(not(test))]
// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
use crate::{math::*, prefab::*};
use js_sys::Array;
use kurbo::{BezPath, PathEl, Point, Rect, Size, Vec2};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TextPos {
    Pos1(f64),
    Pos2(f64),
    Pos3(f64),
    Pos4(f64),
    Pos5(f64),
    PosCustom(Vec2),
}
impl TextPos {
    pub fn pos(&self) -> Vec2 {
        const OFFSET: f64 = 10.;
        match self {
            TextPos::Pos1(y0) => Vec2::new(10., *y0 - OFFSET),
            TextPos::Pos2(y0) => Vec2::new(10., *y0 - 2. * OFFSET),
            TextPos::Pos3(y0) => Vec2::new(10., *y0 - 3. * OFFSET),
            TextPos::Pos4(y0) => Vec2::new(10., *y0 - 4. * OFFSET),
            TextPos::Pos5(y0) => Vec2::new(10., *y0 - 5. * OFFSET),
            TextPos::PosCustom(pos) => *pos,
        }
    }
}
impl Default for TextPos {
    fn default() -> Self {
        TextPos::Pos1(10.)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TextAlign {
    Left,
    Right,
    Center,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasTextConfig {
    color: Color,
    angle: f64,
    align: TextAlign,
    font_size: u32,
    opacity: f64,
}
impl CanvasTextConfig {
    pub fn new(
        color: Color,
        angle: f64,
        align: TextAlign,
        font_size: u32,
        opacity: f64,
    ) -> CanvasTextConfig {
        CanvasTextConfig {
            color,
            angle,
            align,
            font_size,
            opacity,
        }
    }
    pub fn set_angle(&mut self, angle: f64) {
        self.angle = angle
    }
    pub fn set_align(&mut self, align: TextAlign) {
        self.align = align
    }
    pub fn set_font_size(&mut self, font_size: u32) {
        self.font_size = font_size
    }
    pub fn set_opacity(&mut self, opacity: f64) {
        self.opacity = opacity
    }
    pub fn get_angle(&self) -> f64 {
        self.angle
    }
    pub fn get_align(&self) -> TextAlign {
        self.align
    }
    pub fn get_font_size(&self) -> u32 {
        self.font_size
    }
    pub fn get_opacity(&self) -> f64 {
        self.opacity
    }
}
impl Default for CanvasTextConfig {
    fn default() -> Self {
        CanvasTextConfig {
            color: Color::Text,
            angle: 0.,
            align: TextAlign::Center,
            font_size: 16,
            opacity: 0.4,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct CanvasText {
    text: String,
    pos: TextPos,
    config: CanvasTextConfig,
}
impl CanvasText {
    pub fn new(text: String, pos: TextPos, config: CanvasTextConfig) -> CanvasText {
        CanvasText { text, pos, config }
    }
    pub fn set_text(&mut self, text: String) {
        self.text = text
    }
    pub fn set_pos(&mut self, pos: TextPos) {
        self.pos = pos
    }
    pub fn set_config(&mut self, config: CanvasTextConfig) {
        self.config = config
    }
    pub fn get_text(&self) -> &str {
        &self.text
    }
    pub fn get_pos(&self) -> TextPos {
        self.pos
    }
    pub fn get_config(&self) -> &CanvasTextConfig {
        &self.config
    }
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
    ) -> (CanvasKind, BezPath, Pattern, Colors, Vec<CanvasText>) {
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
                texts.push(CanvasText::new(
                    format!("{:.0}", wx / 10.),
                    TextPos::PosCustom(Vec2::new(wx, offset_y + 15.)),
                    CanvasTextConfig::new(Color::Rules, 0., TextAlign::Center, 16, 0.4),
                ));
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
                texts.push(CanvasText::new(
                    format!("{:.0}", wy / 10.),
                    TextPos::PosCustom(Vec2::new(offset_x + 5., wy)),
                    CanvasTextConfig::new(Color::Rules, 0., TextAlign::Left, 16, 0.4),
                ));
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
            BezPath::from_vec(v),
            Pattern::Rules,
            Colors {
                color: Color::Rules,
                fill_color: Color::Transparent,
            },
            texts,
        )
    }

    pub fn draw_grid_primary(
        &mut self,
        draw_rec_size: Size,
        draw_rec_grid_spacing: f64,
    ) -> (CanvasKind, BezPath, Pattern, Colors, Vec<CanvasText>) {
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
            BezPath::from_vec(v),
            Pattern::GridPrimary,
            Colors {
                color: Color::GridPrimary,
                fill_color: Color::Transparent,
            },
            vec![],
        )
    }

    pub fn draw_grid_secondary(
        &mut self,
        draw_rec_size: Size,
        draw_rec_grid_spacing: f64,
    ) -> (CanvasKind, BezPath, Pattern, Colors, Vec<CanvasText>) {
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
            BezPath::from_vec(v),
            Pattern::GridSecondary,
            Colors {
                color: Color::GridSecondary,
                fill_color: Color::Transparent,
            },
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
}
impl Canvases {
    pub fn new(
        // window: Window,
        c_back: HtmlCanvasElement,
        c_grid: HtmlCanvasElement,
        c_main: HtmlCanvasElement,
        drawing_size: Size,
    ) -> Result<Canvases, JsValue> {
        // let document = window.document().expect("should have a document on window");

        // let document_element = document
        //     .document_element()
        //     .ok_or("should have a document element")?;
        // let css_styles = window
        //     .get_computed_style(&document_element)
        //     .unwrap()
        //     .unwrap();
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
            drawing_offset_saved: Vec2::ZERO,
            drawing_offset: Vec2::ZERO,
            drawing_scale: 1.,
            grid_rules: GridRules::new(),
            grid_size: 10.,
            grid_snap: 1.,
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
        let (canvas_kind, path, pattern, colors, texts) = self.grid_rules.draw_rules(
            self.get_drawing_size(),
            self.get_grid_size(),
            self.drawing_offset,
            self.drawing_scale,
        );
        self.draw_path(&canvas_kind, &path, pattern, colors, texts);

        // Primary grid
        let (canvas_kind, path, pattern, colors, texts) = self
            .grid_rules
            .draw_grid_primary(self.get_drawing_size(), self.get_grid_size());
        self.draw_path(&canvas_kind, &path, pattern, colors, texts);

        // Secondary grid
        let (canvas_kind, path, pattern, colors, texts) = self
            .grid_rules
            .draw_grid_secondary(self.get_drawing_size(), self.get_grid_size());
        self.draw_path(&canvas_kind, &path, pattern, colors, texts);
    }

    pub fn clear_main_canvas(&mut self) {
        self.c_main_ctx.clear_rect(
            0.,
            0.,
            self.c_main.width() as f64,
            self.c_main.height() as f64,
        );
    }
    pub fn draw_origin(&self) {
        self.c_grid_ctx.clear_rect(
            0.,
            0.,
            self.c_main.width() as f64,
            self.c_main.height() as f64,
        );
        let origin = to_draw(self.drawing_offset, self.drawing_scale, self.drawing_offset);
        self.draw_path(
            &CanvasKind::Grid,
            &helper_point_path(origin, 5.),
            Pattern::Rules,
            Colors {
                color: Color::Rules,
                fill_color: Color::Transparent,
            },
            vec![],
        );
    }

    pub fn draw_text(&self, canvas_kind: &CanvasKind, text: &CanvasText) {
        let ctx = self.get_context(canvas_kind);
        let scale = self.get_drawing_scale();
        let offset = self.get_drawing_offset();

        ctx.save();

        let cpt = to_canvas(text.pos.pos(), scale, offset);

        ctx.translate(cpt.x, cpt.y)
            .expect("Failed to translate canvas");

        ctx.rotate(text.config.angle)
            .expect("Failed to rotate canvas");
        ctx.set_font(&format!("{}px Ubuntu Mono", text.config.font_size));
        ctx.set_global_alpha(text.config.opacity);

        ctx.set_stroke_style_str(text.config.color.get());
        ctx.set_fill_style_str(text.config.color.get());
        ctx.set_text_align(match text.config.align {
            TextAlign::Left => "left",
            TextAlign::Right => "right",
            TextAlign::Center => "center",
        });
        ctx.fill_text(&text.text, 0., 0.)
            .expect("Failed to draw text");
        ctx.restore();
    }

    pub fn direct_text(&self, canvas_kind: &CanvasKind, text: &CanvasText) {
        let ctx = self.get_context(canvas_kind);
        ctx.save();
        ctx.translate(text.pos.pos().x, text.pos.pos().y)
            .expect("Failed to translate canvas");

        ctx.rotate(text.config.angle)
            .expect("Failed to rotate canvas");
        ctx.set_font(&format!("{}px Ubuntu Mono", text.config.font_size));
        ctx.set_global_alpha(text.config.opacity);

        ctx.set_stroke_style_str(text.config.color.get());
        ctx.set_fill_style_str(text.config.color.get());
        ctx.set_text_align(match text.config.align {
            TextAlign::Left => "left",
            TextAlign::Right => "right",
            TextAlign::Center => "center",
        });
        ctx.fill_text(&text.text, 0., 0.)
            .expect("Failed to draw text");
        ctx.restore();
    }

    pub fn draw_path(
        &self,
        canvas_kind: &CanvasKind,
        path: &BezPath,
        pattern: Pattern,
        colors: Colors,
        texts: Vec<CanvasText>,
    ) {
        let ctx: &CanvasRenderingContext2d = self.get_context(&canvas_kind);
        let scale = self.get_drawing_scale();
        let offset = self.get_drawing_offset();

        let (stroke_style, stroke_width, filled) = pattern.get();
        ctx.set_line_dash(&stroke_style).unwrap();
        ctx.set_line_width(stroke_width);

        ctx.set_fill_style_str(&colors.fill_color.get());
        ctx.set_stroke_style_str(&colors.color.get());

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
        color: Color,
        fill_color: Color,
        texts: Vec<CanvasText>,
    ) {
        let ctx = self.get_context(&canvas_kind);
        let scale = self.get_drawing_scale();
        let offset = self.get_drawing_offset();

        let (stroke_style, stroke_width, filled) = pattern.get();
        ctx.set_line_dash(&stroke_style).unwrap();
        ctx.set_line_width(stroke_width);

        ctx.set_fill_style_str(&fill_color.get());
        ctx.set_stroke_style_str(&color.get());

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

        let (stroke_style, stroke_width, _) = Pattern::Point.get();
        ctx.set_line_dash(&stroke_style).unwrap();
        ctx.set_line_width(stroke_width);

        let color = Color::Red30Opacity.get();
        ctx.set_fill_style_str(&color);
        ctx.set_stroke_style_str(&color);

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
    pub fn reset_origin(&mut self) {
        self.drawing_offset = Vec2::new(
            (self.c_main.width() / 2) as f64,
            (self.c_main.height() / 2) as f64,
        );
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
    pub fn get_canvas_infos(&self) -> (Rect, f64, Vec2) {
        let tl = self.get_canvas_offset();
        let br = Vec2::new(
            tl.x + self.c_main.width() as f64,
            tl.y + self.c_main.height() as f64,
        );
        (
            Rect::from_points(tl.to_point(), br.to_point()),
            self.drawing_scale,
            self.drawing_offset,
        )
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Colors {
    pub color: Color,
    pub fill_color: Color,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Color {
    Background,
    Transparent,
    GridPrimary,
    GridSecondary,
    Rules,
    OnCreation,

    White40Opacity,

    White80Opacity,
    GreenA,

    Red30Opacity,
    Gray,
    White,
    Purple20Opacity,
    Purple55Opacity,
    Pink30Opacity,
    Red60Opacity,
    Text,
    Gray20Opacity,
    Gray95Opacity,
    Olive60Opacity,
    Black65Opacity,
    Black,
    Red,
}
impl Color {
    pub fn get(self) -> &'static str {
        use Color::*;
        match self {
            Background => "rgba(34,68,85,1)",
            Transparent => "rgba(255,255,255,0.125)",
            GridPrimary => "rgba(240,240,240,1)",
            GridSecondary => "rgba(224,224,224,1)",
            Rules => "hsl(350, 68.90%, 52.20%)",
            OnCreation => "rgba(0,119,255,1)",
            White40Opacity => "rgba(255,255,255,0.4)",
            White80Opacity => "rgba(240,240,240,0.8)",
            GreenA => "rgba(128,191,36,1)",
            Red60Opacity => "rgba(255,0,0,0.55)",
            Gray => "rgba(107,114,128,1)",
            White => "rgba(255,255,255,1)",
            Purple20Opacity => "rgba(128,0,128,0.20)",
            Purple55Opacity => "rgba(128,0,128,0.55)",
            Pink30Opacity => "rgba(255,192,203,0.3)",
            Red30Opacity => "rgba(255,0,0,0.3)",
            Text => "rgba(128,128,0,1)",
            Gray20Opacity => "rgba(128,128,128,0.20)",
            Gray95Opacity => "rgba(210, 209, 209, 0.95)",
            Olive60Opacity => "rgba(128,128,0,0.6)",
            Black65Opacity => "rgba(0,0,0,0.65)",
            Black => "rgba(0,0,0,1)",
            Red => "rgba(255,0,0,1)",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Pattern {
    GridPrimary,
    GridSecondary,
    Rules,
    OnCreation,
    Point,
    Composed(bool),
    Basic,
    Helper,
    Text,
    Dim,
}
impl Pattern {
    pub fn get(&self) -> (JsValue, f64, bool) {
        use Pattern::*;
        let dash_pattern = Array::new();
        dash_pattern.push(&JsValue::from_f64(4.0));
        dash_pattern.push(&JsValue::from_f64(6.0));
        let solid_pattern = Array::new();
        let pattern_dashed = JsValue::from(dash_pattern);
        let pattern_solid = JsValue::from(solid_pattern);

        let (line_dash, line_width, filled) = match self {
            GridPrimary => (pattern_solid, 1., false),
            GridSecondary => (pattern_solid, 1., false),
            Rules => (pattern_solid, 1., false),
            OnCreation => (pattern_dashed, 1., true),
            Point => (pattern_solid, 1., true),
            Composed(filled) => (pattern_solid, 3., *filled),
            Basic => (pattern_dashed, 1., false),
            Helper => (pattern_dashed, 1., false),
            Text => (pattern_solid, 1., false),
            Dim => (pattern_solid, 1., false),
        };
        (line_dash, line_width, filled)
    }
}
