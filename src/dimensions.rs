use std::f64::consts::PI;

use kurbo::{BezPath, Vec2};

use crate::{
    canvas::{CanvasText, CanvasTextConfig, Pattern, TextAlign, TextPos},
    curves::curves_edge::SegInfo,
    math::*,
};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DimKind {
    Radius,
    Linear,
    Horizontal,
    Vertical,
    Angle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dimension {
    kind: DimKind,
    start: Vec2,
    end: Vec2,
    dim_offset: f64,
    value: f64,
    highlighted: bool,
    selected: bool,
}
impl Dimension {
    pub fn new(kind: DimKind, start: Vec2, end: Vec2, value: f64) -> Self {
        Self {
            kind,
            start,
            end,
            dim_offset: 2.,
            value,
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
    pub fn get_text_pos(&self) -> TextPos {
        let center = self.get_center();
        let angle = self.get_angle();
        let length = self.get_length();
        let text_pos = center + Vec2::from_angle(angle) * length / 2.;
        TextPos::PosCustom(text_pos)
    }
    pub fn get_text_angle(&self) -> f64 {
        let angle = self.get_angle();
        if angle.abs() > PI / 2. {
            angle + PI
        } else {
            angle
        }
    }
    pub fn get_text_align(&self) -> TextAlign {
        let angle = self.get_angle();
        if angle > std::f64::consts::PI / 2. && angle < 3. * std::f64::consts::PI / 2. {
            TextAlign::Right
        } else {
            TextAlign::Left
        }
    }
    pub fn get_text_val(&self) -> String {
        format!("{:.2}", self.get_length())
    }
    pub fn get_text(&self) -> CanvasText {
        CanvasText::new(
            self.get_text_val(),
            self.get_text_pos(),
            CanvasTextConfig::new(
                self.get_text_pattern(self.selected, self.highlighted),
                self.get_text_angle(),
                self.get_text_align(),
                14,
                1.,
            ),
        )
    }
    pub fn get_path_and_pattern(&self) -> (BezPath, Pattern, CanvasText) {
        match self.kind {
            DimKind::Radius => self.get_radius_path(),
            DimKind::Linear => self.get_linear_path(),
            DimKind::Horizontal => self.get_horizontal_path(),
            DimKind::Vertical => self.get_vertical_path(),
            DimKind::Angle => self.get_angle_path(),
        }
    }
    fn get_text_pattern(&self, selected: bool, highlighted: bool) -> Pattern {
        match (selected, highlighted) {
            (false, false) => Pattern::DimensionTextNormal,
            (false, true) => Pattern::DimensionTextHighlighted,
            (true, false) => Pattern::DimensionTextSelected,
            (true, true) => Pattern::DimensionTextSelected,
        }
    }
    fn get_horizontal_path(&self) -> (BezPath, Pattern, CanvasText) {
        let mut path = kurbo::BezPath::new();
        let value = if self.value == 0. {
            self.get_length()
        } else {
            self.value
        };
        let pos_y = self.start.y - 10.;
        let text = CanvasText::new(
            format!("{:.2}", value),
            TextPos::PosCustom(Vec2::new(self.get_center().x, pos_y - 2.)),
            CanvasTextConfig::new(
                self.get_text_pattern(false, false),
                0.,
                TextAlign::Center,
                14,
                0.5,
            ),
        );

        let start = Vec2::new(self.start.x, pos_y);
        let end = Vec2::new(self.end.x, pos_y);

        path.move_to(Vec2::new(start.x - 2., start.y - 2.).to_point());
        path.line_to(Vec2::new(start.x + 2., start.y + 2.).to_point());

        path.move_to(start.to_point());
        path.line_to(end.to_point());

        path.move_to(Vec2::new(end.x - 2., end.y - 2.).to_point());
        path.line_to(Vec2::new(end.x + 2., end.y + 2.).to_point());

        (path, Pattern::DimensionNormal, text)
    }
    fn get_vertical_path(&self) -> (BezPath, Pattern, CanvasText) {
        let mut path = kurbo::BezPath::new();
        let value = if self.value == 0. {
            self.get_length()
        } else {
            self.value
        };
        let pos_x = self.start.x - 10.;
        let text = CanvasText::new(
            format!("{:.2}", value),
            TextPos::PosCustom(Vec2::new(self.get_center().x - 12., self.get_center().y)),
            CanvasTextConfig::new(
                self.get_text_pattern(false, false),
                -PI / 2.,
                TextAlign::Center,
                14,
                0.5,
            ),
        );

        let start = Vec2::new(pos_x, self.start.y);
        let end = Vec2::new(pos_x, self.end.y);

        path.move_to(Vec2::new(start.x - 2., start.y - 2.).to_point());
        path.line_to(Vec2::new(start.x + 2., start.y + 2.).to_point());

        path.move_to(start.to_point());
        path.line_to(end.to_point());

        path.move_to(Vec2::new(end.x - 2., end.y - 2.).to_point());
        path.line_to(Vec2::new(end.x + 2., end.y + 2.).to_point());

        (path, Pattern::DimensionNormal, text)
    }
    fn get_radius_path(&self) -> (BezPath, Pattern, CanvasText) {
        let mut path = kurbo::BezPath::new();
        let end = self.end + (self.end - self.start).normalize() * 10.;
        let value = if self.value == 0. {
            self.get_length()
        } else {
            self.value
        };
        let text = CanvasText::new(
            format!("r: {:.2}", value),
            TextPos::PosCustom(Vec2::new(end.x + 2., end.y - 2.)),
            CanvasTextConfig::new(
                self.get_text_pattern(false, false),
                0.,
                TextAlign::Left,
                14,
                0.5,
            ),
        );

        path.move_to(self.start.to_point());

        path.line_to(end.to_point());
        path.line_to(Vec2::new(end.x + 10., end.y).to_point());

        (path, Pattern::DimensionNormal, text)
    }
    fn get_angle_path(&self) -> (BezPath, Pattern, CanvasText) {
        let mut path = kurbo::BezPath::new();
        let start = self.start;
        let end = self.end;
        let unit_vec = (end - start).normalize();
        let ratio = self.get_length() * 0.4;
        let angle_pt = start + unit_vec * ratio;
        let angle = (self.end - self.start).atan2();
        let text_pos_angle = get_point_at_dist_from_angle(start, angle / 2., 10.);

        let text = CanvasText::new(
            format!("a: {:.1}", -angle / PI * 180.),
            TextPos::PosCustom(text_pos_angle),
            CanvasTextConfig::new(
                self.get_text_pattern(false, false),
                0., //self.get_text_angle(),
                TextAlign::Left,
                14,
                0.5,
            ),
        );

        path.move_to((start + Vec2::new(ratio, 0.)).to_point());
        path.line_to(start.to_point());
        path.line_to(angle_pt.to_point());

        (path, Pattern::DimensionNormal, text)
    }
    fn get_linear_path(&self) -> (BezPath, Pattern, CanvasText) {
        use Pattern::*;
        let mut path = BezPath::new();
        let value = if self.value < EPSILON {
            self.get_length()
        } else {
            self.value
        };
        let seg_info = SegInfo::new(self.start, self.end);
        if let Some(seg) = seg_info {
            let unit_rot45 = rotate_vector(seg.end() - seg.start(), PI / 4.).normalize();
            let text = CanvasText::new(
                format!("{:.1}", value),
                TextPos::PosCustom(self.get_center() + seg.n_dir() * (self.dim_offset + 10.)),
                CanvasTextConfig::new(
                    self.get_text_pattern(false, false),
                    self.get_text_angle(),
                    TextAlign::Center,
                    14,
                    0.5,
                ),
            );

            path.move_to((seg.start() + seg.n_dir() * self.dim_offset).to_point());
            path.line_to((seg.end() + seg.n_dir() * self.dim_offset).to_point());

            path.move_to(
                (seg.start() + seg.n_dir() * self.dim_offset - 2. * unit_rot45).to_point(),
            );
            path.line_to(
                (seg.start() + seg.n_dir() * self.dim_offset + 2. * unit_rot45).to_point(),
            );

            path.move_to((seg.end() + seg.n_dir() * self.dim_offset - 2. * unit_rot45).to_point());
            path.line_to((seg.end() + seg.n_dir() * self.dim_offset + 2. * unit_rot45).to_point());

            (path, DimensionNormal, text)
        } else {
            (
                path,
                DimensionNormal,
                CanvasText::new(
                    "".into(),
                    TextPos::PosCustom(Vec2::new(0., 0.)),
                    CanvasTextConfig::new(DimensionNormal, 0., TextAlign::Center, 14, 0.5),
                ),
            )
        }
    }
}
