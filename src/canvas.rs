use crate::{
    clipboard::Clipboard,
    dimensions::{dim_classic, dim_disc},
    helpers::{math::*, prefab::*},
    inputs::{SystemMouse, UserAction, UserUI},
    shape::{GeneralShape, ShapeType, SvgFillRule},
    shapes::DataSet,
    types::vertex::Vertex,
    types::others::{EUId, Property, PropertyValue},
    undoredo::{DrawState, UndoRedo},
};
use js_sys::Array;
use kurbo::{BezPath, PathEl, Point, Rect, Shape, Size, Vec2};
use std::mem;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, CanvasWindingRule, HtmlCanvasElement, MouseEvent};

#[derive(Debug, Clone)]
pub struct Note {
    pub id: usize,
    pub pos: Vec2,
    pub size: Vec2,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct NotesData {
    pub notes: Vec<Note>,
    next_id: usize,
}
impl Default for NotesData {
    fn default() -> Self {
        Self::new()
    }
}

impl NotesData {
    pub fn new() -> Self {
        Self {
            notes: Vec::new(),
            next_id: 0,
        }
    }
    pub fn clear(&mut self) {
        self.notes.clear();
        self.next_id = 0;
    }
    pub fn add(&mut self, pos: Vec2, size: Vec2, text: String) -> usize {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.notes.push(Note {
            id,
            pos,
            size,
            text,
        });
        id
    }
    pub fn add_with_id(&mut self, id: usize, pos: Vec2, size: Vec2, text: String) {
        self.next_id = self.next_id.max(id.saturating_add(1));
        self.notes.push(Note {
            id,
            pos,
            size,
            text,
        });
    }
    pub fn get_mut(&mut self, id: usize) -> Option<&mut Note> {
        self.notes.iter_mut().find(|note| note.id == id)
    }
    pub fn remove(&mut self, id: usize) -> Option<Note> {
        let idx = self.notes.iter().position(|note| note.id == id)?;
        Some(self.notes.remove(idx))
    }
}

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
    Toolpath = 4,
}

impl CanvasKind {
    pub const COUNT: usize = 5;

    #[inline(always)]
    pub const fn idx(self) -> usize {
        self as usize
    }
}

#[allow(dead_code)]
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
    pub fn _draw_rules(
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
    pub fn _draw_grid_primary(
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
    pub fn _draw_grid_secondary(
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
    pub notes: NotesData,
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
            notes: NotesData::new(),
            clipboard: Clipboard::new(),
            undo_redo: UndoRedo::new(),
        })
    }

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
    pub fn set_area_size(&mut self, size: Size) {
        self.area_size = size;
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
        let delta = self.user_ui.pointer.curr() - self.user_ui.pointer.saved();
        self.offset = to_canvas(delta, self.scale, self.offset);
    }
    pub fn save_offset(&mut self) {
        self.offset_saved = self.offset
    }
    pub fn snapshot_draw_state(&self) -> DrawState {
        DrawState {
            dataset: self.dataset.clone(),
            notes: self.notes.clone(),
            scale: self.scale,
            offset: self.offset,
        }
    }
    pub fn restore_draw_state(&mut self, state: DrawState) {
        self.dataset = state.dataset;
        self.notes = state.notes;
        self.scale = state.scale;
        self.offset = state.offset;
    }
    pub fn begin_history_action(&mut self) {
        let before = self.snapshot_draw_state();
        self.undo_redo.begin(before);
    }
    pub fn commit_history_action(&mut self) {
        let after = self.snapshot_draw_state();
        self.undo_redo.commit(after);
    }
    pub fn abort_history_action(&mut self) {
        self.undo_redo.abort();
    }
    pub fn push_history_action(&mut self, before: DrawState) {
        let after = self.snapshot_draw_state();
        self.undo_redo.push(before, after);
    }
    pub fn undo_history(&mut self) -> bool {
        let current = self.snapshot_draw_state();
        if let Some(state) = self.undo_redo.undo(current) {
            self.restore_draw_state(state);
            return true;
        }
        false
    }
    pub fn redo_history(&mut self) -> bool {
        let current = self.snapshot_draw_state();
        if let Some(state) = self.undo_redo.redo(current) {
            self.restore_draw_state(state);
            return true;
        }
        false
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
    pub fn set_cursor(&self, cursor: &str) {
        let _ = self.canvas.style().set_property("cursor", cursor);
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
    pub fn move_vertices_selected(&mut self) -> Option<()> {
        let (eid, vid) = self.dataset.vertex_selected?;
        let is_group = self
            .dataset
            .get_element(eid)
            .map(GeneralShape::is_group)
            .unwrap_or(false);
        if is_group {
            if self.dataset.scale_group_by_vertex(eid, vid, &self.user_ui) {
                return Some(());
            }
            return None;
        }
        self.dataset
            .get_element_mut(eid)?
            .move_vertex_by_mouse(vid, &self.user_ui)?;

        let e = self.dataset.get_element_mut(eid)?;
        if let ShapeType::ConstrCircle = e.get_shape_type() {
            if let Some(PropertyValue::Magnets { value: magnets, .. }) =
                e.get_properties().get(&Property::Magnets).cloned()
            {
                let vs = e.get_vertices_mut();
                let v1 = vs.val(0).curr();
                let v2 = vs.val(1).curr();
                let vs_magnets = get_magnets_vertices(v1, v2, magnets.curr());
                for (idx, (_, v)) in e.get_vertices_mut().iter_mut().skip(2).enumerate() {
                    *v = Vertex::new(vs_magnets[idx]);
                }
                self.dataset.mark_final_polygon_dirty();
            }
        }

        Some(())
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
            .set_font(&format!("{}px Urbanist", text.config.font_size));
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
            .set_font(&format!("{}px Urbanist", text.config.font_size));
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

        self.ctx.set_fill_style_str(fill_color.get());
        self.ctx.set_stroke_style_str(stroke_color.get());

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
    pub fn draw_paths(
        &self,
        paths: &[BezPath],
        pattern: Pattern,
        fill_color: Color,
        stroke_color: Color,
    ) {
        for path in paths.iter() {
            self.draw_path(path, pattern, fill_color, stroke_color, vec![]);
        }
    }
    pub fn draw_svg_paths(
        &self,
        paths: &[BezPath],
        pattern: Pattern,
        fill_color: Color,
        stroke_color: Color,
        fill_rule: SvgFillRule,
    ) {
        let (stroke_style, stroke_width, filled) = pattern.get();
        self.ctx.set_line_dash(&stroke_style).unwrap();
        self.ctx.set_line_width(stroke_width);

        self.ctx.set_fill_style_str(fill_color.get());
        self.ctx.set_stroke_style_str(stroke_color.get());

        self.ctx.begin_path();
        for path in paths {
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
            match fill_rule {
                SvgFillRule::EvenOdd => {
                    self.ctx
                        .fill_with_canvas_winding_rule(CanvasWindingRule::Evenodd);
                }
                SvgFillRule::NonZero => self.ctx.fill(),
            }
        }
        self.ctx.stroke();
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

        self.ctx.set_fill_style_str(fill_color.get());
        self.ctx.set_stroke_style_str(color.get());

        self.ctx.begin_path();
        for path in paths.iter() {
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
        self.ctx.set_fill_style_str(color);
        self.ctx.set_stroke_style_str(color);

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
        self.draw_rules_only(self.scale, self.offset);
    }
    pub fn draw_rules_only(&mut self, drawing_scale: f64, drawing_offset: Vec2) {
        self.clear();

        let spacing = self.get_grid_size();
        if spacing <= EPSILON {
            return;
        }

        let canvas_size = self.get_canvas_size();
        let (stroke_style, stroke_width, _) = Pattern::Rules.get();
        self.ctx.set_line_dash(&stroke_style).unwrap();
        self.ctx.set_line_width(stroke_width);

        let color = Color::Rules.get();
        self.ctx.set_stroke_style_str(color);

        let draw_min = to_draw(Vec2::ZERO, drawing_scale, drawing_offset);
        let draw_max = to_draw(
            Vec2::new(canvas_size.width, canvas_size.height),
            drawing_scale,
            drawing_offset,
        );

        let major_spacing = spacing * 10.0;
        let mut labels: Vec<CanvasText> = Vec::new();
        let text_cfg = CanvasTextConfig::new(Color::Rules, 0., TextAlign::Center, 16, 0.4);
        let text_cfg_left = CanvasTextConfig::new(Color::Rules, 0., TextAlign::Left, 16, 0.4);

        let mut x = (draw_min.x / spacing).floor() * spacing;
        let x_end = draw_max.x + spacing;
        self.ctx.begin_path();
        self.ctx.move_to(0., 0.);
        self.ctx.line_to(canvas_size.width, 0.);
        while x <= x_end {
            let is_major = ((x / major_spacing).round() - x / major_spacing).abs() < 1e-6;
            let tick = if is_major {
                self.grid_rules.primary_rules_thicks_hw
            } else {
                self.grid_rules.secondary_rules_thicks_hw
            };
            let cx = to_canvas(Vec2::new(x, 0.0), drawing_scale, drawing_offset).x;
            self.ctx.move_to(cx, 0.);
            self.ctx.line_to(cx, tick);
            if is_major {
                labels.push(CanvasText::new(
                    format!("{:.0}", x),
                    TextPos::PosCustom(Vec2::new(cx, tick + 12.)),
                    text_cfg.clone(),
                ));
            }
            x += spacing;
        }

        let mut y = (draw_min.y / spacing).floor() * spacing;
        let y_end = draw_max.y + spacing;
        self.ctx.move_to(0., 0.);
        self.ctx.line_to(0., canvas_size.height);
        while y <= y_end {
            let is_major = ((y / major_spacing).round() - y / major_spacing).abs() < 1e-6;
            let tick = if is_major {
                self.grid_rules.primary_rules_thicks_hw
            } else {
                self.grid_rules.secondary_rules_thicks_hw
            };
            let cy = to_canvas(Vec2::new(0.0, y), drawing_scale, drawing_offset).y;
            self.ctx.move_to(0., cy);
            self.ctx.line_to(tick, cy);
            if is_major {
                labels.push(CanvasText::new(
                    format!("{:.0}", y),
                    TextPos::PosCustom(Vec2::new(tick + 4., cy + 5.)),
                    text_cfg_left.clone(),
                ));
            }
            y += spacing;
        }
        self.ctx.stroke();

        for text in labels.iter() {
            self.direct_text(text);
        }
    }
    pub fn may_be_draw_radiuses(&mut self) {
        let text_color = get_text_colors().stroke_color;
        let text_cfg = CanvasTextConfig::new(text_color, 0., TextAlign::Center, 14, 0.8);
        for e in self.dataset.shapes.values() {
            match e.get_shape_type() {
                ShapeType::Square | ShapeType::Poly => {
                    let nc = e.get_n_corners();
                    let apices = if e.get_shape_type() == ShapeType::Poly {
                        e.get_vertices().get_apices_poly(nc)
                    } else {
                        e.get_vertices().get_apices_n(nc)
                    };
                    // Fillet radii at corners
                    for apex_type in apices.iter() {
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
                    // Sag arc radii on edges
                    if e.get_vertices().len() > nc {
                        self.draw_sag_arc_radii(e, nc, &text_cfg);
                    }
                }
                ShapeType::Oblong => {
                    // Sag arc radii for oblong sides
                    self.draw_sag_arc_radii_oblong(e, &text_cfg);
                }
                _ => (),
            }
        }
    }
    /// Draw sag arc radius labels for Rectangle/Polygon edges.
    fn draw_sag_arc_radii(&self, e: &GeneralShape, nc: usize, text_cfg: &CanvasTextConfig) {
        use crate::helpers::math::{poly_sag_from_vertex, ApexType};
        use crate::shape::solve_side;
        let verts = e.get_vertices();
        let scale = self.get_scale();
        let offset = 12.0 / scale;
        let apices = if e.get_shape_type() == ShapeType::Poly {
            verts.get_apices_poly(nc)
        } else {
            verts.get_apices_n(nc)
        };

        let entry_of = |apex: &ApexType| -> Vec2 {
            match *apex { ApexType::TypeVertex { a } => a, ApexType::Arc { s, .. } => s }
        };
        let exit_of = |apex: &ApexType| -> Vec2 {
            match *apex { ApexType::TypeVertex { a } => a, ApexType::Arc { e, .. } => e }
        };

        for i in 0..nc {
            let j = (i + 1) % nc;
            let sag_idx = nc + i;
            if sag_idx >= verts.len() { continue; }
            let edge_start = exit_of(&apices[i]);
            let edge_end = entry_of(&apices[j]);
            let handle = verts.val(sag_idx as i64).curr();
            let sag = poly_sag_from_vertex(edge_start, edge_end, handle);
            if sag.abs() < 1e-6 { continue; }

            let fillet_i = match apices[i] { ApexType::Arc { c, .. } => Some((c, (exit_of(&apices[i]) - c).hypot())), _ => None };
            let fillet_j = match apices[j] { ApexType::Arc { c, .. } => Some((c, (entry_of(&apices[j]) - c).hypot())), _ => None };

            let radius = match (fillet_i, fillet_j) {
                (Some((c1, r1)), Some((c2, r2))) => {
                    solve_side(c1, c2, r1, r2, handle, edge_start, edge_end).map(|(_, _, _, r)| r)
                }
                (Some((c1, r1)), None) => {
                    solve_side(c1, edge_end, r1, 0.0, handle, edge_start, edge_end).map(|(_, _, _, r)| r)
                }
                (None, Some((c2, r2))) => {
                    solve_side(edge_start, c2, 0.0, r2, handle, edge_start, edge_end).map(|(_, _, _, r)| r)
                }
                (None, None) => {
                    crate::helpers::math::circle_from_three_points(edge_start, handle, edge_end)
                        .map(|(_, r)| r)
                }
            };

            if let Some(radius) = radius {
                let chord = edge_end - edge_start;
                let l = chord.hypot();
                if l < 1e-9 { continue; }
                let n = Vec2::new(-chord.y / l, chord.x / l);
                // Text baseline is at the top of the text; add font height when text is below
                let dir = n * sag.signum();
                let goes_down = dir.y > 0.0; // positive y = downward in screen coords
                let extra = if goes_down { 14.0 / scale } else { 0.0 };
                let text_pos = handle + dir * (offset + extra);
                let text2 = CanvasText::new(
                    format!("R{:.0}", radius),
                    TextPos::PosCustom(text_pos),
                    text_cfg.clone(),
                );
                self.draw_text(&text2);
            }
        }
    }

    /// Draw sag arc radius labels for Oblong sides.
    fn draw_sag_arc_radii_oblong(&self, e: &GeneralShape, text_cfg: &CanvasTextConfig) {
        use crate::helpers::math::poly_sag_from_vertex;
        use crate::shape::{oblong_straight_angles, solve_side};
        let verts = e.get_vertices();
        if verts.len() < 6 { return; }
        let scale = self.get_scale();
        let offset = 12.0 / scale;
        let c1 = verts.val(0).curr();
        let c2 = verts.val(1).curr();
        let r1 = (verts.val(2).curr() - c1).hypot();
        let r2 = (verts.val(3).curr() - c2).hypot();
        let Some((a1_s, a2_s)) = oblong_straight_angles(c1, c2, r1, r2) else { return };

        for (idx, angle) in [(4, a2_s), (5, a1_s)] {
            let handle = verts.val(idx as i64).curr();
            let p1 = c1 + Vec2::new(r1 * angle.cos(), r1 * angle.sin());
            let p2 = c2 + Vec2::new(r2 * angle.cos(), r2 * angle.sin());
            let sag = poly_sag_from_vertex(p1, p2, handle);
            if sag.abs() < 1e-6 { continue; }

            if let Some((_, _, _, radius)) = solve_side(c1, c2, r1, r2, handle, p1, p2) {
                let chord = p2 - p1;
                let l = chord.hypot();
                if l < 1e-9 { continue; }
                let n = Vec2::new(-chord.y / l, chord.x / l);
                // Text baseline is at the top of the text; add font height when text is below
                let dir = n * sag.signum();
                let goes_down = dir.y > 0.0; // positive y = downward in screen coords
                let extra = if goes_down { 14.0 / scale } else { 0.0 };
                let text_pos = handle + dir * (offset + extra);
                let text2 = CanvasText::new(
                    format!("R{:.0}", radius),
                    TextPos::PosCustom(text_pos),
                    text_cfg.clone(),
                );
                self.draw_text(&text2);
            }
        }
    }

    pub fn draw_dimensions(&mut self, e: &GeneralShape) {
        match e.get_shape_type() {
            ShapeType::Disc => {
                let v: Vec<Vec2> = e.get_vertices().iter().map(|(_, v)| v.curr()).collect();
                if v.len() >= 2 {
                    let center = v[0];
                    let radius = (v[1] - center).hypot();
                    if radius > 0.01 {
                        // dim_offsets[0] stores the angle; -20.0 = sentinel "use natural direction"
                        let stored = e.get_dim_offsets()[0];
                        let natural = (v[1] - center).y.atan2((v[1] - center).x);
                        let angle = if (stored - (-20.0)).abs() < 1e-9 { natural } else { stored };
                        let cinfo = self.get_canvas_infos();
                        let (path, pattern, colors, text, _handle) =
                            dim_disc(center, radius, angle, cinfo);
                        self.draw_path(&path, pattern, colors.fill_color, colors.stroke_color, text);
                    }
                }
            }
            ShapeType::ConstrCircle | ShapeType::ConstrLine => {}
            ShapeType::Square
            | ShapeType::Text
            | ShapeType::Svg
            | ShapeType::Voronoi
            | ShapeType::Group => {
                let points: Vec<Vec2> = e.get_vertices().iter().map(|(_, v)| v.curr()).collect();
                if points.len() < 4 {
                    return;
                }
                let rotation = e.get_rotation();
                let dim_offsets = e.get_dim_offsets();
                let cinfo = self.get_canvas_infos();

                // H dimension: top edge v[0]→v[3], V dimension: right edge v[2]→v[3]
                let dim_configs = [
                    (points[0], points[3], dim_offsets[0], format!("{:.0}", (points[3] - points[0]).hypot())),
                    (points[2], points[3], dim_offsets[1], format!("{:.0}", (points[3] - points[2]).hypot())),
                ];

                // Angle label (shown once, near v[0], when rotated)
                let mut angle_text: Option<CanvasText> = if rotation.abs() > EPSILON {
                    let mut deg = rotation.to_degrees();
                    while deg <= -180.0 { deg += 360.0; }
                    while deg > 180.0 { deg -= 360.0; }
                    let angle_pos = points[0]
                        + rotate_vector(Vec2::new(10.0, -10.0) / self.scale, rotation);
                    Some(CanvasText::new(
                        format!("A{deg:.1}°"),
                        TextPos::PosCustom(angle_pos),
                        CanvasTextConfig::new(
                            get_text_colors().stroke_color,
                            rotation,
                            TextAlign::Left,
                            14,
                            0.8,
                        ),
                    ))
                } else {
                    None
                };

                let bbox = e.get_bezpath().bounding_box();
                let center = Vec2::new((bbox.x0 + bbox.x1) * 0.5, (bbox.y0 + bbox.y1) * 0.5);

                for (p1, p2, offset, label) in dim_configs.iter() {
                    let (mut path, pattern, colors, mut text, _handle) =
                        dim_classic(*p1, *p2, *offset, label.clone(), cinfo);
                    if rotation.abs() > EPSILON {
                        path = rotate_bezpath_about(&path, center, rotation);
                        text = text
                            .into_iter()
                            .map(|mut item| {
                                if let TextPos::PosCustom(pos) = item.get_pos() {
                                    let rotated = rotate_vector(pos - center, rotation) + center;
                                    item.set_pos(TextPos::PosCustom(rotated));
                                }
                                let mut cfg = item.get_config().clone();
                                cfg.set_angle(cfg.get_angle() + rotation);
                                item.set_config(cfg);
                                item
                            })
                            .collect();
                    }
                    // Append angle label to first dim's text (consumed once)
                    if let Some(atxt) = angle_text.take() {
                        text.push(atxt);
                    }
                    self.draw_path(&path, pattern, colors.fill_color, colors.stroke_color, text);
                }
                // Corner radii (Square only) — show dim for each active corner
                if e.get_shape_type() == ShapeType::Square {
                    let nc = e.get_n_corners();
                    for (i, (_, v)) in e.get_vertices().iter().enumerate() {
                        if i >= nc { break; }
                        let Some(r_val) = v.get_radius().filter(|&r| r > 0) else { continue };
                        let r_f = r_val as f64;
                        if i >= points.len() { continue; }
                        let corner = points[i];
                        let sx = if center.x > corner.x { 1.0 } else { -1.0 };
                        let sy = if center.y > corner.y { 1.0 } else { -1.0 };
                        let arc_center = corner + Vec2::new(sx * r_f, sy * r_f);
                        let natural_angle =
                            (corner - arc_center).y.atan2((corner - arc_center).x);
                        let stored = e.get_dim_offsets()[2];
                        let angle = if (stored - (-20.0)).abs() < 1e-9 {
                            natural_angle
                        } else {
                            stored
                        };
                        let (mut path, pattern, colors, mut text, _) =
                            dim_disc(arc_center, r_f, angle, cinfo);
                        if rotation.abs() > EPSILON {
                            path = rotate_bezpath_about(&path, center, rotation);
                            text = text
                                .into_iter()
                                .map(|mut item| {
                                    if let TextPos::PosCustom(pos) = item.get_pos() {
                                        let rotated =
                                            rotate_vector(pos - center, rotation) + center;
                                        item.set_pos(TextPos::PosCustom(rotated));
                                    }
                                    let mut cfg = item.get_config().clone();
                                    cfg.set_angle(cfg.get_angle() + rotation);
                                    item.set_config(cfg);
                                    item
                                })
                                .collect();
                        }
                        self.draw_path(
                            &path,
                            pattern,
                            colors.fill_color,
                            colors.stroke_color,
                            text,
                        );
                    }
                }
                // Sag arc radii (Square)
                let nc = e.get_n_corners();
                if e.get_shape_type() == ShapeType::Square && e.get_vertices().len() > nc {
                    let text_cfg = CanvasTextConfig::new(get_text_colors().stroke_color, 0., TextAlign::Center, 14, 0.8);
                    self.draw_sag_arc_radii(e, nc, &text_cfg);
                }
            }
            ShapeType::Oblong => {
                let v: Vec<Vec2> = e.get_vertices().iter().map(|(_, v)| v.curr()).collect();
                if v.len() < 4 {
                    return;
                }
                let cinfo = self.get_canvas_infos();
                let dim_offset = e.get_dim_offsets()[0];
                // Axis length: classic dim on v[0]→v[1]
                let axis = v[1] - v[0];
                let length_label = format!("{:.1}", axis.hypot());
                let (path, pattern, colors, text, _) =
                    dim_classic(v[0], v[1], dim_offset, length_label, cinfo);
                self.draw_path(&path, pattern, colors.fill_color, colors.stroke_color, text);
                // Radius at v[0] (control vertex v[2]) — dim_offsets[1] stores the angle
                {
                    let r0 = (v[2] - v[0]).hypot();
                    let natural0 = (v[2] - v[0]).y.atan2((v[2] - v[0]).x);
                    let stored0 = e.get_dim_offsets()[1];
                    let a0 = if (stored0 - (-20.0)).abs() < 1e-9 { natural0 } else { stored0 };
                    let (path, pattern, colors, text, _) = dim_disc(v[0], r0, a0, cinfo);
                    self.draw_path(&path, pattern, colors.fill_color, colors.stroke_color, text);
                }
                // Radius at v[1] (control vertex v[3]) — dim_offsets[2] stores the angle
                {
                    let r1 = (v[3] - v[1]).hypot();
                    let natural1 = (v[3] - v[1]).y.atan2((v[3] - v[1]).x);
                    let stored1 = e.get_dim_offsets()[2];
                    let a1 = if (stored1 - (-20.0)).abs() < 1e-9 { natural1 } else { stored1 };
                    let (path, pattern, colors, text, _) = dim_disc(v[1], r1, a1, cinfo);
                    self.draw_path(&path, pattern, colors.fill_color, colors.stroke_color, text);
                }
                // Sag arc radii
                let text_cfg = CanvasTextConfig::new(get_text_colors().stroke_color, 0., TextAlign::Center, 14, 0.8);
                self.draw_sag_arc_radii_oblong(e, &text_cfg);
            }
            ShapeType::Poly => {
                let verts = e.get_vertices();
                if verts.len() < 3 {
                    return;
                }
                let bbox = e.get_bezpath().bounding_box();
                if bbox.width() < 0.1 && bbox.height() < 0.1 {
                    return;
                }
                let tl = Vec2::new(bbox.x0, bbox.y0);
                let tr = Vec2::new(bbox.x1, bbox.y0);
                let br = Vec2::new(bbox.x1, bbox.y1);
                let dim_offsets = e.get_dim_offsets();
                let cinfo = self.get_canvas_infos();
                // H : arête du haut tl→tr
                let (path, pattern, colors, text, _) = dim_classic(
                    tl, tr, dim_offsets[0],
                    format!("{:.0}", bbox.width()), cinfo,
                );
                self.draw_path(&path, pattern, colors.fill_color, colors.stroke_color, text);
                // V : arête droite br→tr
                let (path, pattern, colors, text, _) = dim_classic(
                    br, tr, dim_offsets[1],
                    format!("{:.0}", bbox.height()), cinfo,
                );
                self.draw_path(&path, pattern, colors.fill_color, colors.stroke_color, text);
                // Rayons des apex arrondis
                let apices = verts.get_apices_poly(e.get_n_corners());
                let mut first_arc = true;
                for apex in apices.iter() {
                    if let ApexType::Arc { s, c, .. } = apex {
                        let radius = (*s - *c).hypot();
                        if radius < 0.01 {
                            continue;
                        }
                        let natural_angle = (*s - *c).y.atan2((*s - *c).x);
                        let angle = if first_arc {
                            let stored = dim_offsets[2];
                            if (stored - (-20.0)).abs() < 1e-9 {
                                natural_angle
                            } else {
                                stored
                            }
                        } else {
                            natural_angle
                        };
                        let (path, pattern, colors, text, _) =
                            dim_disc(*c, radius, angle, cinfo);
                        self.draw_path(
                            &path,
                            pattern,
                            colors.fill_color,
                            colors.stroke_color,
                            text,
                        );
                        first_arc = false;
                    }
                }
                // Sag arc radii
                let nc = e.get_n_corners();
                if e.get_vertices().len() > nc {
                    let text_cfg = CanvasTextConfig::new(get_text_colors().stroke_color, 0., TextAlign::Center, 14, 0.8);
                    self.draw_sag_arc_radii(e, nc, &text_cfg);
                }
            }
            ShapeType::Arrow => (),
        }
    }

    pub fn draw_vs(&mut self, e: &GeneralShape, selected: bool) {
        let scale = self.get_scale();
        for (vid, vertex) in e.get_vertices().iter() {
            let pos = e.vertex_display_pos(vertex.curr());
            let vid_sel = self
                .dataset
                .vertex_selected
                .iter()
                .any(|&(_, sel_vid)| &sel_vid == vid);
            let vid_high = self
                .dataset
                .vertex_highlighted
                .iter()
                .any(|&(_, high_vid)| &high_vid == vid);

            let colors = get_vertices_colors(vid_sel, vid_high);
            if vid_sel {
                let ring = point_path(pos, scale * 0.45);
                self.draw_path(
                    &ring,
                    Pattern::Composed(false),
                    Color::Transparent,
                    Color::Red,
                    vec![],
                );
            } else if vid_high {
                let ring = point_path(pos, scale * 0.45);
                self.draw_path(
                    &ring,
                    Pattern::Composed(false),
                    Color::Transparent,
                    Color::Green40,
                    vec![],
                );
            }
            self.draw_path(
                &point_path(pos, scale),
                Pattern::Point,
                colors.fill_color,
                colors.stroke_color,
                vec![],
            );
        }
        // Rotation handle (outside v[2]) for rotatable shapes
        if let Some(h) = e.rotation_handle_world_pos(scale) {
            let ring = point_path(h, scale);
            self.draw_path(
                &ring,
                Pattern::Point,
                Color::Ocre,
                Color::Black,
                vec![],
            );
        }
        // Ghost handle display (only when shape is selected)
        if selected && e.ghost().is_active() {
            use crate::shape::GhostMode;
            let h = e.ghost().handle;
            let ring = point_path(h, scale);
            self.draw_path(&ring, Pattern::Point, Color::Ocre, Color::Black, vec![]);
            match e.ghost().mode {
                GhostMode::Rotation => {
                    let sz = 6.0 / scale;
                    let mut cross = BezPath::new();
                    cross.move_to(kurbo::Point::new(h.x - sz, h.y));
                    cross.line_to(kurbo::Point::new(h.x + sz, h.y));
                    cross.move_to(kurbo::Point::new(h.x, h.y - sz));
                    cross.line_to(kurbo::Point::new(h.x, h.y + sz));
                    self.draw_path(&cross, Pattern::Composed(false), Color::Transparent, Color::Red30, vec![]);
                }
                GhostMode::MirrorH => {
                    // ↔ reflect across vertical axis at x=H.x
                    let ext = 2000.0;
                    let mut line = BezPath::new();
                    line.move_to(kurbo::Point::new(h.x, -ext));
                    line.line_to(kurbo::Point::new(h.x, ext));
                    self.draw_path(&line, Pattern::Composed(false), Color::Transparent, Color::Red30, vec![]);
                }
                GhostMode::MirrorV => {
                    // ↕ reflect across horizontal axis at y=H.y
                    let ext = 2000.0;
                    let mut line = BezPath::new();
                    line.move_to(kurbo::Point::new(-ext, h.y));
                    line.line_to(kurbo::Point::new(ext, h.y));
                    self.draw_path(&line, Pattern::Composed(false), Color::Transparent, Color::Red30, vec![]);
                }
                GhostMode::Copy => {
                    let center = e.bbox_center_pub();
                    let sc = kurbo::Point::new(
                        (center.x / 10.0).round() * 10.0,
                        (center.y / 10.0).round() * 10.0,
                    );
                    let mut line = BezPath::new();
                    line.move_to(sc);
                    line.line_to(kurbo::Point::new(h.x, h.y));
                    self.draw_path(&line, Pattern::Composed(false), Color::Transparent, Color::Red30, vec![]);
                }
                _ => {}
            }
        }
    }
    pub fn draw_vertices(&mut self) {
        // move shapes out of self.dataset temporarily
        let shapes = mem::take(&mut self.dataset.shapes);

        let selected = self.dataset.shapes_selected.clone();
        let highlighted = self.dataset.shapes_highlighted.clone();
        for (eid, e) in shapes.iter() {
            let is_sel = selected.contains(eid);
            self.draw_vs(e, is_sel);
            if is_sel || highlighted.contains(eid) {
                self.draw_dimensions(e);
            }
        }
        // put back
        self.dataset.shapes = shapes;
    }

    pub fn draw_paths_creation(&mut self, e: &GeneralShape) {
        let stroke_color = get_stroke_color(false, false);
        let path = e.get_bezpath();
        self.draw_path(
            path,
            Pattern::OnCreation,
            Color::Gray95,
            stroke_color,
            vec![],
        );
    }
    pub fn draw_user_dimensions(&mut self, selected_id: Option<usize>) {
        use crate::dimensions::{dim_classic, dim_disc};
        use crate::shapes::DimKind;
        let dims = self.dataset.user_dims.clone();
        for dim in &dims {
            let cinfo = self.get_canvas_infos();
            let stroke_override = if Some(dim.id) == selected_id {
                Some(Color::Red)
            } else {
                None
            };
            if dim.kind == DimKind::Radius {
                let Some(info) = self.dataset.resolve_radius_info(dim.eid1, dim.vid1) else {
                    continue;
                };
                let (path, pattern, colors, text, _) =
                    dim_disc(info.center, info.radius, dim.offset, cinfo);
                let stroke = stroke_override.unwrap_or(colors.stroke_color);
                self.draw_path(&path, pattern, colors.fill_color, stroke, text);
                continue;
            }
            let p1 = self.dataset.resolve_vertex_pos(dim.eid1, dim.vid1);
            let p2 = self.dataset.resolve_vertex_pos(dim.eid2, dim.vid2);
            let (Some(p1), Some(p2)) = (p1, p2) else {
                continue;
            };
            let (dp1, dp2, label) = match dim.kind {
                DimKind::Horizontal => {
                    let y = (p1.y + p2.y) * 0.5;
                    (
                        Vec2::new(p1.x, y),
                        Vec2::new(p2.x, y),
                        format!("{:.1}", (p2.x - p1.x).abs()),
                    )
                }
                DimKind::Vertical => {
                    let x = (p1.x + p2.x) * 0.5;
                    (
                        Vec2::new(x, p1.y),
                        Vec2::new(x, p2.y),
                        format!("{:.1}", (p2.y - p1.y).abs()),
                    )
                }
                DimKind::Aligned => (p1, p2, format!("{:.1}", (p2 - p1).hypot())),
                DimKind::Radius => continue,
            };
            let (path, pattern, colors, text, _handle) =
                dim_classic(dp1, dp2, dim.offset, label, cinfo);
            let stroke = stroke_override.unwrap_or(colors.stroke_color);
            self.draw_path(&path, pattern, colors.fill_color, stroke, text);
        }
    }
    pub fn draw_paths_sets(&mut self) {
        self.draw_paths_sets_with_svg_bbox(false);
    }
    fn draw_shape_paths_with_state(
        &mut self,
        shape: &GeneralShape,
        is_selected: bool,
        is_highlighted: bool,
        svg_bbox_only: bool,
    ) {
        let is_voronoi = shape.get_shape_type() == ShapeType::Voronoi;
        let is_construction = matches!(
            shape.get_shape_type(),
            ShapeType::ConstrLine | ShapeType::ConstrCircle
        );
        let stroke_color = if !is_selected && !is_highlighted {
            Color::Gray20
        } else {
            get_stroke_color(is_selected, is_highlighted)
        };
        let fill_color = if is_construction {
            Color::Transparent
        } else if svg_bbox_only && shape.get_shape_type() == ShapeType::Svg {
            if is_selected || is_highlighted {
                get_fill_color(is_selected, is_highlighted)
            } else {
                Color::Transparent
            }
        } else {
            get_fill_color(is_selected, is_highlighted)
        };
        let pattern = if is_construction {
            Pattern::Helper
        } else {
            Pattern::Point
        };
        if shape.get_shape_type() == ShapeType::Text {
            if let Some(paths) = shape.get_text_paths_rotated() {
                self.draw_svg_paths(
                    &paths,
                    pattern,
                    fill_color,
                    stroke_color,
                    SvgFillRule::NonZero,
                );
            } else {
                let path = shape.get_bezpath_rotated();
                self.draw_path(&path, pattern, fill_color, stroke_color, vec![]);
            }
            return;
        }
        if matches!(shape.get_shape_type(), ShapeType::Svg | ShapeType::Voronoi) {
            if svg_bbox_only && !is_voronoi {
                let path = shape.get_bezpath_rotated();
                self.draw_path(&path, pattern, fill_color, stroke_color, vec![]);
                return;
            }
            if let (Some(paths), Some(svg)) = (shape.get_svg_paths_rotated(), shape.get_svg()) {
                self.draw_svg_paths(&paths, pattern, fill_color, stroke_color, svg.fill_rule);
            } else {
                let path = shape.get_bezpath_rotated();
                self.draw_path(&path, pattern, fill_color, stroke_color, vec![]);
            }
            // fall through to draw ghosts below
        } else {
            let path = shape.get_bezpath_rotated();
            self.draw_path(&path, pattern, fill_color, stroke_color, vec![]);
        }
        // Draw ghost copies (apply shape rotation for display)
        let ghost_paths = shape.get_ghost_bezpaths();
        if !ghost_paths.is_empty() {
            let rotation = shape.get_rotation();
            let needs_rot = rotation.abs() > 1e-9 && matches!(
                shape.get_shape_type(),
                ShapeType::Square | ShapeType::Text | ShapeType::Svg | ShapeType::Voronoi | ShapeType::Group
            );
            for ghost in ghost_paths {
                let ghost = if needs_rot {
                    let bbox = shape.get_bezpath().bounding_box();
                    let center = Vec2::new((bbox.x0 + bbox.x1) * 0.5, (bbox.y0 + bbox.y1) * 0.5);
                    crate::shape::rotate_bezpath_pub(&ghost, center, rotation)
                } else {
                    ghost
                };
                self.draw_path(&ghost, Pattern::Composed(false), Color::Gray20, Color::Gray20, vec![]);
            }
        }
    }
    fn draw_group_paths(
        &mut self,
        group: &GeneralShape,
        is_selected: bool,
        is_highlighted: bool,
        svg_bbox_only: bool,
    ) {
        let child_ids = group.get_group_children().unwrap_or_default().to_vec();
        for child_id in child_ids {
            let Some(child) = self.dataset.grouped_shapes.get(&child_id).cloned() else {
                continue;
            };
            if child.is_group() {
                self.draw_group_paths(&child, is_selected, is_highlighted, svg_bbox_only);
            } else {
                self.draw_shape_paths_with_state(
                    &child,
                    is_selected,
                    is_highlighted,
                    svg_bbox_only,
                );
            }
        }
        if is_selected || is_highlighted {
            let stroke_color = get_stroke_color(is_selected, is_highlighted);
            let path = group.get_bezpath_rotated();
            self.draw_path(
                &path,
                Pattern::Point,
                Color::Transparent,
                stroke_color,
                vec![],
            );
        }
    }
    pub fn draw_paths_sets_with_svg_bbox(&mut self, svg_bbox_only: bool) {
        let selected = self.dataset.shapes_selected.clone();
        let highlighted = self.dataset.shapes_highlighted.clone();
        let shape_ids: Vec<EUId> = self.dataset.shapes.keys().copied().collect();
        for eid in shape_ids {
            let Some(shape) = self.dataset.shapes.get(&eid).cloned() else {
                continue;
            };
            let is_selected = selected.contains(&eid);
            let is_highlighted = highlighted.contains(&eid);
            if shape.is_group() {
                self.draw_group_paths(&shape, is_selected, is_highlighted, svg_bbox_only);
                continue;
            }
            self.draw_shape_paths_with_state(&shape, is_selected, is_highlighted, svg_bbox_only);
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
    Ocre,
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
            Ocre => "rgba(204,153,0,1)",
        }
    }
}

fn rotate_bezpath_about(path: &BezPath, center: Vec2, angle: f64) -> BezPath {
    let mut out = BezPath::new();
    for elem in path.iter() {
        match elem {
            PathEl::MoveTo(pt) => {
                out.push(PathEl::MoveTo(rotate_point_about(pt, center, angle)));
            }
            PathEl::LineTo(pt) => {
                out.push(PathEl::LineTo(rotate_point_about(pt, center, angle)));
            }
            PathEl::QuadTo(pt1, pt2) => {
                out.push(PathEl::QuadTo(
                    rotate_point_about(pt1, center, angle),
                    rotate_point_about(pt2, center, angle),
                ));
            }
            PathEl::CurveTo(pt1, pt2, pt3) => {
                out.push(PathEl::CurveTo(
                    rotate_point_about(pt1, center, angle),
                    rotate_point_about(pt2, center, angle),
                    rotate_point_about(pt3, center, angle),
                ));
            }
            PathEl::ClosePath => out.push(PathEl::ClosePath),
        }
    }
    out
}

fn rotate_point_about(point: Point, center: Vec2, angle: f64) -> Point {
    let v = Vec2::new(point.x, point.y);
    let rotated = rotate_vector(v - center, angle) + center;
    Point::new(rotated.x, rotated.y)
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
            Composed(filled) => (pattern_solid, 2.5, *filled),
            Basic => (pattern_dashed, 1., false),
            Helper => (pattern_dashed, 1., false),
            Text => (pattern_solid, 1., false),
            Dim => (pattern_solid, 1., true),
        };
        (line_dash, line_width, filled)
    }
}
