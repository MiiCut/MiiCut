use std::f64::consts::PI;

use kurbo::{BezPath, Vec2};

use crate::{
    canvas::{Align, CanvasText, Pattern},
    math::*,
};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DimKind {
    Radius,
    Linear,
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dimension {
    kind: DimKind,
    start: Vec2,
    end: Vec2,
    dim_offset: f64,
    highlighted: bool,
    selected: bool,
}
impl Dimension {
    pub fn new(kind: DimKind, start: Vec2, end: Vec2) -> Self {
        Self {
            kind,
            start,
            dim_offset: 2.,
            end,
            highlighted: false,
            selected: false,
        }
    }
    pub fn get_dim_kind(&self) -> DimKind {
        self.kind
    }
    pub fn get_start(&self) -> Vec2 {
        self.start
    }
    pub fn get_end(&self) -> Vec2 {
        self.end
    }
    pub fn set_start(&mut self, start: Vec2) {
        self.start = start;
    }
    pub fn set_end(&mut self, end: Vec2) {
        self.end = end;
    }
    pub fn set_dim_offset(&mut self, dim_offset: f64) {
        self.dim_offset = dim_offset;
    }
    pub fn get_length(&self) -> f64 {
        (self.start - self.end).hypot()
    }
    pub fn get_angle(&self) -> f64 {
        (self.end - self.start).angle()
    }
    pub fn get_center(&self) -> Vec2 {
        (self.start + self.end) / 2.
    }
    pub fn get_text_pos(&self) -> Vec2 {
        let center = self.get_center();
        let angle = self.get_angle();
        let length = self.get_length();
        let text_pos = center + Vec2::from_angle(angle) * length / 2.;
        text_pos
    }
    pub fn get_text_angle(&self) -> f64 {
        let mut angle = self.get_angle();
        if angle < -PI / 2. || angle > PI / 2. {
            angle = PI + angle;
        }
        angle
    }
    pub fn get_text_align(&self) -> Align {
        let angle = self.get_angle();
        if angle > std::f64::consts::PI / 2. && angle < 3. * std::f64::consts::PI / 2. {
            Align::Right
        } else {
            Align::Left
        }
    }
    pub fn get_text_val(&self) -> String {
        format!("{:.2}", self.get_length())
    }
    pub fn get_pattern(&self, selected: bool, highlighted: bool) -> Pattern {
        match (selected, highlighted) {
            (false, false) => Pattern::DimensionNormal,
            (false, true) => Pattern::DimensionHighlighted,
            (true, false) => Pattern::DimensionSelected,
            (true, true) => Pattern::DimensionSelected,
        }
    }
    pub fn get_text(&self) -> CanvasText {
        CanvasText {
            text: self.get_text_val(),
            pos: self.get_text_pos(),
            pattern: self.get_pattern(self.selected, self.highlighted),
            angle: self.get_text_angle(),
            align: self.get_text_align(),
            font_size: 14,
            opacity: 1.,
        }
    }
    pub fn get_path(&self) -> ((BezPath, Pattern), CanvasText) {
        match self.kind {
            DimKind::Radius => self.get_radius_path(),
            DimKind::Linear => self.get_linear_path(),
            DimKind::Horizontal => self.get_horizontal_path(),
            DimKind::Vertical => self.get_vertical_path(),
        }
    }
    fn get_horizontal_path(&self) -> ((BezPath, Pattern), CanvasText) {
        let mut path = kurbo::BezPath::new();
        let length = self.get_length();
        let pos_y = self.start.y - 10.;
        let text = CanvasText {
            text: format!("{:.0}", length),
            pos: Vec2::new(self.get_center().x, pos_y - 2.),
            pattern: Pattern::DimensionNormal,
            angle: 0.,
            align: Align::Center,
            font_size: 14,
            opacity: 0.5,
        };

        let start = Vec2::new(self.start.x, pos_y);
        let end = Vec2::new(self.end.x, pos_y);

        path.move_to(Vec2::new(start.x - 2., start.y - 2.).to_point());
        path.line_to(Vec2::new(start.x + 2., start.y + 2.).to_point());

        path.move_to(start.to_point());
        path.line_to(end.to_point());

        path.move_to(Vec2::new(end.x - 2., end.y - 2.).to_point());
        path.line_to(Vec2::new(end.x + 2., end.y + 2.).to_point());

        ((path, Pattern::DimensionNormal), text)
    }
    fn get_vertical_path(&self) -> ((BezPath, Pattern), CanvasText) {
        let mut path = kurbo::BezPath::new();
        let length = self.get_length();
        let pos_x = self.start.x - 10.;
        let text = CanvasText {
            text: format!("{:.0}", length),
            pos: Vec2::new(self.get_center().x - 12., self.get_center().y),
            pattern: Pattern::DimensionNormal,
            angle: -PI / 2.,
            align: Align::Center,
            font_size: 14,
            opacity: 0.5,
        };

        let start = Vec2::new(pos_x, self.start.y);
        let end = Vec2::new(pos_x, self.end.y);

        path.move_to(Vec2::new(start.x - 2., start.y - 2.).to_point());
        path.line_to(Vec2::new(start.x + 2., start.y + 2.).to_point());

        path.move_to(start.to_point());
        path.line_to(end.to_point());

        path.move_to(Vec2::new(end.x - 2., end.y - 2.).to_point());
        path.line_to(Vec2::new(end.x + 2., end.y + 2.).to_point());

        ((path, Pattern::DimensionNormal), text)
    }
    fn get_radius_path(&self) -> ((BezPath, Pattern), CanvasText) {
        let mut path = kurbo::BezPath::new();
        let length = self.get_length();
        let text = CanvasText {
            text: format!("{:.0}", length),
            pos: Vec2::new(self.end.x + 2., self.end.y - 2.),
            pattern: Pattern::DimensionNormal,
            angle: 0.,
            align: Align::Left,
            font_size: 14,
            opacity: 0.5,
        };

        path.move_to(self.start.to_point());
        path.line_to(self.end.to_point());
        path.line_to(Vec2::new(self.end.x + 10., self.end.y).to_point());

        ((path, Pattern::DimensionNormal), text)
    }
    fn get_linear_path(&self) -> ((BezPath, Pattern), CanvasText) {
        let mut path = kurbo::BezPath::new();
        let length = self.get_length();
        let start = self.start;
        let end = self.end;
        let unit_perp = unit_perpendicular(start, end, false);
        let unit_rot45 = rotate_vector(end - start, PI / 4.).normalize();
        let text = CanvasText {
            text: format!("{:.1}", length),
            pos: self.get_center() + unit_perp * (self.dim_offset + 2.),
            pattern: Pattern::DimensionNormal,
            angle: self.get_text_angle(),
            align: Align::Center,
            font_size: 14,
            opacity: 0.5,
        };

        path.move_to((start + unit_perp * self.dim_offset).to_point());
        path.line_to((end + unit_perp * self.dim_offset).to_point());

        path.move_to((start + unit_perp * self.dim_offset - 2. * unit_rot45).to_point());
        path.line_to((start + unit_perp * self.dim_offset + 2. * unit_rot45).to_point());

        path.move_to((end + unit_perp * self.dim_offset - 2. * unit_rot45).to_point());
        path.line_to((end + unit_perp * self.dim_offset + 2. * unit_rot45).to_point());

        ((path, Pattern::DimensionNormal), text)
    }
}
