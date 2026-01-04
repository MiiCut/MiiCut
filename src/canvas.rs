use std::mem;

// #![cfg(not(test))]
// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
use crate::{
    clipboard::Clipboard,
    dimensions::{dim_hv, dim_radius},
    dom::Icons,
    inputs::{SystemMouse, UserAction, UserUI},
    math::*,
    prefab::*,
    shape::ClosedShape,
    shapes::DataSet,
    types::{Binding, Couple, EUId, SegBundle, VUId},
    undoredo::UndoRedo,
};
use js_sys::Array;
use kurbo::{BezPath, PathEl, Point, Rect, Size, Vec2};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, MouseEvent};

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

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CanvasKind {
    Background = 0,
    Grid = 1,
    Draw = 2,
    Gcode = 3,
}

impl CanvasKind {
    pub const COUNT: usize = 4;

    #[inline(always)]
    pub const fn idx(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Debug)]
struct GridRules {
    h_rule_height: f64,
    v_rule_width: f64,
    primary_rules_thicks_hw: f64,
    secondary_rules_thicks_hw: f64,

    grid_size: f64,
    grid_snap: f64,
}
impl GridRules {
    fn new() -> GridRules {
        GridRules {
            h_rule_height: 0.,
            v_rule_width: 0.,
            primary_rules_thicks_hw: 8.,
            secondary_rules_thicks_hw: 4.,
            grid_size: 10.,
            grid_snap: 1.,
        }
    }
    pub fn draw_rules(
        &mut self,
        draw_rec_size: Size,
        draw_rec_grid_spacing: f64,
        drawing_offset: Vec2,
        drawing_scale: f64,
    ) -> (BezPath, Pattern, Colors, Vec<CanvasText>) {
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
            BezPath::from_vec(v),
            Pattern::Rules,
            Colors {
                stroke_color: Color::Rules,
                fill_color: Color::Transparent,
            },
            texts,
        )
    }
    pub fn draw_grid_primary(
        &mut self,
        draw_rec_size: Size,
        draw_rec_grid_spacing: f64,
    ) -> (BezPath, Pattern, Colors, Vec<CanvasText>) {
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
            BezPath::from_vec(v),
            Pattern::GridPrimary,
            Colors {
                stroke_color: Color::GridPrimary,
                fill_color: Color::Transparent,
            },
            vec![],
        )
    }
    pub fn draw_grid_secondary(
        &mut self,
        draw_rec_size: Size,
        draw_rec_grid_spacing: f64,
    ) -> (BezPath, Pattern, Colors, Vec<CanvasText>) {
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
            BezPath::from_vec(v),
            Pattern::GridSecondary,
            Colors {
                stroke_color: Color::GridSecondary,
                fill_color: Color::Transparent,
            },
            vec![],
        )
    }
}

#[derive(Debug)]
pub struct Canvas {
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    // The size (mm,mm) = (px,px) of the drawing area
    area_size: Size,
    offset_saved: Vec2,
    offset: Vec2,
    scale: f64,
    //
    grid_rules: GridRules,
    // user interface
    user_ui: UserUI,
    pointer_on_canvas: bool,
    //
    pub dataset: DataSet,
    pub clipboard: Clipboard,
    pub undo_redo: UndoRedo,
}
impl Canvas {
    pub fn new(html_canvas: HtmlCanvasElement, area_size: Size) -> Result<Canvas, JsValue> {
        let ctx = html_canvas
            .get_context("2d")?
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()?;

        Ok(Canvas {
            canvas: html_canvas,
            ctx,
            area_size,
            offset_saved: Vec2::ZERO,
            offset: Vec2::ZERO,
            scale: 1.,
            grid_rules: GridRules::new(),
            user_ui: UserUI::new(),
            pointer_on_canvas: false,
            dataset: DataSet::new(),
            clipboard: Clipboard::new(),
            undo_redo: UndoRedo::new(),
        })
    }
    // pub fn draw_grid_and_rules(&mut self) {
    //     self.ctx.clear_rect(
    //         0.,
    //         0.,
    //         self.canvas.width() as f64,
    //         self.canvas.height() as f64,
    //     );
    //     // Rules
    //     let (path, pattern, colors, texts) = self.grid_rules.draw_rules(
    //         self.get_size(),
    //         self.get_grid_size(),
    //         self.offset,
    //         self.scale,
    //     );
    //     self.draw_path(
    //         &path,
    //         pattern,
    //         colors.fill_color,
    //         colors.stroke_color,
    //         texts,
    //     );
    //     // Primary grid
    //     let (path, pattern, colors, texts) = self
    //         .grid_rules
    //         .draw_grid_primary(self.get_size(), self.get_grid_size());
    //     self.draw_path(
    //         &path,
    //         pattern,
    //         colors.fill_color,
    //         colors.stroke_color,
    //         texts,
    //     );
    //     // Secondary grid
    //     let (path, pattern, colors, texts) = self
    //         .grid_rules
    //         .draw_grid_secondary(self.get_size(), self.get_grid_size());
    //     self.draw_path(
    //         &path,
    //         pattern,
    //         colors.fill_color,
    //         colors.stroke_color,
    //         texts,
    //     );
    // }

    pub fn get_canvas(&self) -> &HtmlCanvasElement {
        &self.canvas
    }
    pub fn get_context(&self) -> &CanvasRenderingContext2d {
        &self.ctx
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        self.canvas.set_width(width);
        self.canvas.set_height(height);
    }
    pub fn reset_origin(&mut self) {
        self.offset = Vec2::new(
            (self.canvas.width() / 2) as f64,
            (self.canvas.height() / 2) as f64,
        );
    }
    pub fn get_scale(&self) -> f64 {
        self.scale
    }
    pub fn get_size(&self) -> Size {
        self.area_size
    }
    pub fn get_offset(&self) -> Vec2 {
        self.offset
    }
    pub fn set_offset(&mut self, offset: Vec2) {
        self.offset = offset
    }
    pub fn move_offset(&mut self) {
        let delta = self.user_ui.pointer.curr - self.user_ui.pointer.saved;
        self.offset = to_canvas(delta, self.scale, self.offset);
    }
    pub fn save_offset(&mut self) {
        self.offset_saved = self.offset
    }
    pub fn set_scale(&mut self, scale: f64) {
        self.scale = scale
    }
    pub fn set_grid_size(&mut self, size: f64) {
        self.grid_rules.grid_size = size
    }
    pub fn set_grid_snap(&mut self, size: f64) {
        self.grid_rules.grid_snap = size
    }
    pub fn set_pointer_on_canvas(&mut self, on_canvas: bool) {
        self.pointer_on_canvas = on_canvas
    }
    pub fn is_pointer_on_canvas(&self) -> bool {
        self.pointer_on_canvas
    }
    pub fn get_grid_size(&self) -> f64 {
        self.grid_rules.grid_size
    }

    pub fn get_grid_snap(&self) -> f64 {
        self.grid_rules.grid_snap
    }
    pub fn get_canvas_offset(&self) -> Vec2 {
        Vec2::new(
            self.canvas.get_bounding_client_rect().x(),
            self.canvas.get_bounding_client_rect().y(),
        )
    }
    pub fn get_canvas_size(&self) -> Size {
        Size::new(
            self.canvas.get_bounding_client_rect().width(),
            self.canvas.get_bounding_client_rect().height(),
        )
    }
    pub fn get_canvas_infos(&self) -> (Rect, f64, Vec2) {
        let tl = self.get_canvas_offset();
        let br = Vec2::new(
            tl.x + self.canvas.width() as f64,
            tl.y + self.canvas.height() as f64,
        );
        (
            Rect::from_points(tl.to_point(), br.to_point()),
            self.scale,
            self.offset,
        )
    }
    pub fn get_user_ui(&self) -> &UserUI {
        &self.user_ui
    }
    pub fn get_user_ui_mut(&mut self) -> &mut UserUI {
        &mut self.user_ui
    }
    pub fn update_ui(
        &mut self,
        origin: Vec2,
        mouse_event: &MouseEvent,
        sys_mouse: SystemMouse,
    ) -> UserAction {
        self.user_ui
            .update_ui(origin, self.offset, self.scale, mouse_event, sys_mouse)
    }

    // Dataset manipulations
    pub fn select_elements(&mut self) {
        self.dataset.select_elements(&self.user_ui);
    }
    pub fn highlight_elements(&mut self) {
        self.dataset.highlight_elements(&self.user_ui);
    }
    pub fn move_elements(&mut self) -> bool {
        self.dataset.move_elements(&self.user_ui)
    }
    pub fn highlight_vertices(&mut self) -> bool {
        self.dataset.highlight_vertices(self.user_ui.draw_pos)
    }
    pub fn move_vertices_selected(&mut self) -> bool {
        if let Some((last_eid, last_vid)) = self.dataset.last_vertex_selected {
            self.dataset.get_element_mut(last_eid).map(|e| {
                return e.move_vertex(last_vid, &self.user_ui);
            });
        }
        false
    }

    // Rendering
    pub fn clear(&mut self) {
        self.ctx.clear_rect(
            0.,
            0.,
            self.canvas.width() as f64,
            self.canvas.height() as f64,
        );
    }
    pub fn draw_origin(&self) {
        let origin = to_draw(self.offset, self.scale, self.offset);
        self.draw_path(
            &helper_point_path(origin, 5.),
            Pattern::Rules,
            Color::Rules,
            Color::Transparent,
            vec![],
        );
    }
    pub fn draw_text(&self, text: &CanvasText) {
        self.ctx.save();

        let cpt = to_canvas(text.pos.pos(), self.scale, self.offset);

        self.ctx
            .translate(cpt.x, cpt.y)
            .expect("Failed to translate canvas");

        self.ctx
            .rotate(text.config.angle)
            .expect("Failed to rotate canvas");
        self.ctx
            .set_font(&format!("{}px Stencilia", text.config.font_size));
        self.ctx.set_global_alpha(text.config.opacity);

        self.ctx.set_stroke_style_str(text.config.color.get());
        self.ctx.set_fill_style_str(text.config.color.get());
        self.ctx.set_text_align(match text.config.align {
            TextAlign::Left => "left",
            TextAlign::Right => "right",
            TextAlign::Center => "center",
        });
        self.ctx
            .fill_text(&text.text, 0., 0.)
            .expect("Failed to draw text");
        self.ctx.restore();
    }
    pub fn direct_text(&self, text: &CanvasText) {
        self.ctx.save();
        self.ctx
            .translate(text.pos.pos().x, text.pos.pos().y)
            .expect("Failed to translate canvas");

        self.ctx
            .rotate(text.config.angle)
            .expect("Failed to rotate canvas");
        self.ctx
            .set_font(&format!("{}px Stencilia", text.config.font_size));
        self.ctx.set_global_alpha(text.config.opacity);

        self.ctx.set_stroke_style_str(text.config.color.get());
        self.ctx.set_fill_style_str(text.config.color.get());
        self.ctx.set_text_align(match text.config.align {
            TextAlign::Left => "left",
            TextAlign::Right => "right",
            TextAlign::Center => "center",
        });
        self.ctx
            .fill_text(&text.text, 0., 0.)
            .expect("Failed to draw text");
        self.ctx.restore();
    }
    pub fn draw_path(
        &self,
        path: &BezPath,
        pattern: Pattern,
        fill_color: Color,
        stroke_color: Color,
        texts: Vec<CanvasText>,
    ) {
        let (stroke_style, stroke_width, filled) = pattern.get();
        self.ctx.set_line_dash(&stroke_style).unwrap();
        self.ctx.set_line_width(stroke_width);

        self.ctx.set_fill_style_str(&fill_color.get());
        self.ctx.set_stroke_style_str(&stroke_color.get());

        self.ctx.begin_path();
        for cst in path.iter() {
            match cst {
                PathEl::MoveTo(pt) => {
                    let cpt = to_canvas(pt.to_vec2(), self.scale, self.offset);
                    self.ctx.move_to(cpt.x, cpt.y);
                }
                PathEl::LineTo(pt) => {
                    let cpt = to_canvas(pt.to_vec2(), self.scale, self.offset);
                    self.ctx.line_to(cpt.x, cpt.y);
                }
                PathEl::QuadTo(pt1, pt2) => {
                    let cpt1 = to_canvas(pt1.to_vec2(), self.scale, self.offset);
                    let cpt2 = to_canvas(pt2.to_vec2(), self.scale, self.offset);
                    self.ctx.quadratic_curve_to(cpt1.x, cpt1.y, cpt2.x, cpt2.y);
                }
                PathEl::CurveTo(pt1, pt2, pt3) => {
                    let cpt1 = to_canvas(pt1.to_vec2(), self.scale, self.offset);
                    let cpt2 = to_canvas(pt2.to_vec2(), self.scale, self.offset);
                    let cpt3 = to_canvas(pt3.to_vec2(), self.scale, self.offset);
                    self.ctx
                        .bezier_curve_to(cpt1.x, cpt1.y, cpt2.x, cpt2.y, cpt3.x, cpt3.y);
                }
                PathEl::ClosePath => self.ctx.close_path(),
            }
        }
        if filled {
            self.ctx.fill();
        }
        self.ctx.stroke();

        // Drawing the texts
        for text in texts.iter() {
            self.draw_text(text);
        }
    }
    pub fn draw_closed_path(
        &self,
        pattern: Pattern,
        color: Color,
        fill_color: Color,
        texts: Vec<CanvasText>,
    ) {
        let paths = self.dataset.get_final_paths();
        let (stroke_style, stroke_width, filled) = pattern.get();
        self.ctx.set_line_dash(&stroke_style).unwrap();
        self.ctx.set_line_width(stroke_width);

        self.ctx.set_fill_style_str(&fill_color.get());
        self.ctx.set_stroke_style_str(&color.get());

        self.ctx.begin_path();
        for (_idx, path) in paths.iter().enumerate() {
            for cst in path.iter() {
                match cst {
                    PathEl::MoveTo(pt) => {
                        let cpt = to_canvas(pt.to_vec2(), self.scale, self.offset);
                        self.ctx.move_to(cpt.x, cpt.y);
                    }
                    PathEl::LineTo(pt) => {
                        let cpt = to_canvas(pt.to_vec2(), self.scale, self.offset);
                        self.ctx.line_to(cpt.x, cpt.y);
                    }
                    PathEl::QuadTo(pt1, pt2) => {
                        let cpt1 = to_canvas(pt1.to_vec2(), self.scale, self.offset);
                        let cpt2 = to_canvas(pt2.to_vec2(), self.scale, self.offset);
                        self.ctx.quadratic_curve_to(cpt1.x, cpt1.y, cpt2.x, cpt2.y);
                    }
                    PathEl::CurveTo(pt1, pt2, pt3) => {
                        let cpt1 = to_canvas(pt1.to_vec2(), self.scale, self.offset);
                        let cpt2 = to_canvas(pt2.to_vec2(), self.scale, self.offset);
                        let cpt3 = to_canvas(pt3.to_vec2(), self.scale, self.offset);
                        self.ctx
                            .bezier_curve_to(cpt1.x, cpt1.y, cpt2.x, cpt2.y, cpt3.x, cpt3.y);
                    }
                    PathEl::ClosePath => self.ctx.close_path(),
                }
            }
        }
        if filled {
            self.ctx.fill();
        }
        self.ctx.stroke();
        // Drawing the texts
        for text in texts.iter() {
            self.draw_text(text);
        }
    }
    pub fn draw_pointer(&self, position: Vec2) {
        let canvas_size = self.get_canvas_size();
        let pos_canvas = to_canvas(position, self.scale, self.offset);

        let (stroke_style, stroke_width, _) = Pattern::Point.get();
        self.ctx.set_line_dash(&stroke_style).unwrap();
        self.ctx.set_line_width(stroke_width);

        let color = Color::Red30.get();
        self.ctx.set_fill_style_str(&color);
        self.ctx.set_stroke_style_str(&color);

        self.ctx.begin_path();
        self.ctx.move_to(0., pos_canvas.y);
        self.ctx.line_to(canvas_size.width, pos_canvas.y);
        self.ctx.move_to(pos_canvas.x, 0.);
        self.ctx.line_to(pos_canvas.x, canvas_size.height);
        self.ctx.close_path();

        self.ctx.stroke();
    }
    pub fn draw_reset_origin(&mut self) {
        self.reset_origin();
    }
    pub fn draw_grid_and_rules(&mut self) {
        self.draw_origin();
    }
    pub fn may_be_draw_radiuses(&mut self) {
        let text_color = get_text_colors().stroke_color;
        let text_cfg = CanvasTextConfig::new(text_color, 0., TextAlign::Center, 14, 0.8);
        for e in self.dataset.shapes.values() {
            match e.get_shape_type() {
                Icons::Square | Icons::Poly => {
                    for apex_type in e.get_vertices().get_apices().iter() {
                        if let ApexType::Arc { s, c, e: _e } = apex_type {
                            let r = (*s - *c).length();
                            let text2 = CanvasText::new(
                                format!("R{:.0}", r),
                                TextPos::PosCustom(*c),
                                text_cfg.clone(),
                            );
                            self.draw_text(&text2);
                        }
                    }
                }
                _ => (),
            }
        }
    }
    pub fn draw_dimensions(&mut self, e: &ClosedShape) {
        match e.get_shape_type() {
            Icons::Disc => {
                let v: Vec<Vec2> = e.get_vertices().iter().map(|(_, v)| v.curr).collect();
                if v.len() == 2 {
                    if let Some(seg) = SegBundle::new(v[0], v[1]) {
                        // Draw the radius segment
                        let (path, pattern, colors, text) =
                            dim_radius(seg, self.get_canvas_infos(), true, true);
                        self.draw_path(
                            &path,
                            pattern,
                            colors.fill_color,
                            colors.stroke_color,
                            text,
                        );
                    }
                }
            }
            Icons::Square | Icons::Text => {
                for (v1, v2) in e
                    .get_vertices()
                    .iter()
                    .zip(e.get_vertices().iter().cycle().skip(1))
                    .take(2)
                    .map(|(v1, v2)| (v1.1.curr, v2.1.curr))
                {
                    if let Some(seg) = SegBundle::new(v1, v2) {
                        let (path, pattern, colors, text) = dim_hv(seg, self.get_canvas_infos());
                        self.draw_path(
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
                            dim_radius(seg1, self.get_canvas_infos(), false, true);
                        self.draw_path(
                            &path,
                            pattern,
                            colors.fill_color,
                            colors.stroke_color,
                            text,
                        );
                        // Draw the radius segment
                        if let Some(seg2) = SegBundle::new(seg1.m, v[2]) {
                            let (path, pattern, colors, text) =
                                dim_radius(seg2, self.get_canvas_infos(), true, false);
                            self.draw_path(
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
                        let (path, pattern, colors, text) = dim_hv(seg, self.get_canvas_infos());
                        self.draw_path(
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

    pub fn draw_vs(&mut self, e: &ClosedShape) {
        for (vid, vertex) in e.get_vertices().iter() {
            let vid_sel = self
                .dataset
                .vertices_selected
                .iter()
                .any(|&(_, sel_vid)| &sel_vid == vid);
            let vid_high = self
                .dataset
                .vertices_highlighted
                .iter()
                .any(|&(_, high_vid)| &high_vid == vid);

            let colors = get_vertices_colors(vid_sel, vid_high);
            self.draw_path(
                &point_path(vertex.curr, 1.),
                Pattern::Point,
                colors.fill_color,
                colors.stroke_color,
                vec![],
            );
        }
    }
    pub fn draw_vertices(&mut self) -> Binding<(EUId, VUId)> {
        let mut binds = Binding::<(EUId, VUId)>::new();

        // move shapes out of self.dataset temporarily
        let shapes = mem::take(&mut self.dataset.shapes);

        for (_eid, e) in shapes.iter() {
            self.draw_vs(e);
            self.draw_dimensions(e);
            for (vid, vertex) in e.get_vertices().iter() {
                binds.extend(
                    vertex
                        .bind
                        .iter()
                        .map(|(eid2, vid2)| Couple((*eid2, *vid), (*eid2, *vid2))),
                );
            }
        }
        // put back
        self.dataset.shapes = shapes;
        binds
    }
    pub fn draw_paths_creation(&mut self, e: &ClosedShape) {
        let stroke_color = get_stroke_color(false, false);
        let path = e.get_bezpath();
        self.draw_path(
            &path,
            Pattern::OnCreation,
            Color::Gray95,
            stroke_color,
            vec![],
        );
    }
    pub fn draw_paths_sets(&mut self) {
        for (eid, e) in self.dataset.shapes.iter() {
            let stroke_color = get_stroke_color(
                self.dataset.shapes_selected.contains(eid),
                self.dataset.shapes_highlighted.contains(eid),
            );
            let fill_color = get_fill_color(
                self.dataset.shapes_selected.contains(eid),
                self.dataset.shapes_highlighted.contains(eid),
            );
            let path = e.get_bezpath();
            self.draw_path(path, Pattern::Point, fill_color, stroke_color, vec![]);
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Colors {
    pub stroke_color: Color,
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

    White40,

    White80,
    Green40,

    Red30,
    Gray,
    White,
    Purple20,
    Purple55,
    Pink30,
    Red60,
    Text,
    Gray20,
    Gray95,
    Olive60,
    Black65,
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
            White40 => "rgba(255,255,255,0.4)",
            White80 => "rgba(240,240,240,0.8)",
            Green40 => "rgba(128,191,36,0.4)",
            Red60 => "rgba(255,0,0,0.55)",
            Gray => "rgba(107,114,128,1)",
            White => "rgba(255,255,255,1)",
            Purple20 => "rgba(128,0,128,0.20)",
            Purple55 => "rgba(128,0,128,0.55)",
            Pink30 => "rgba(255,192,203,0.3)",
            Red30 => "rgba(255,0,0,0.3)",
            Text => "rgba(128,128,0,1)",
            Gray20 => "rgba(128,128,128,0.20)",
            Gray95 => "rgba(210, 209, 209, 0.95)",
            Olive60 => "rgba(128,128,0,0.6)",
            Black65 => "rgba(0,0,0,0.65)",
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
            Composed(filled) => (pattern_solid, 5., *filled),
            Basic => (pattern_dashed, 1., false),
            Helper => (pattern_dashed, 1., false),
            Text => (pattern_solid, 1., false),
            Dim => (pattern_solid, 1., false),
        };
        (line_dash, line_width, filled)
    }
}
