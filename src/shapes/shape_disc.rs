// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
use super::shapes::{BSKind, BSKindvars};
use crate::{
    canvas::{CanvasText, Pattern},
    dimensions::{DimKind, Dimension},
    math::*,
    positions::{Position, Value, HS},
    prefab::{center_path, modifiers_path},
    traits::*,
};
use geo::{LineString, Polygon};
use kurbo::{BezPath, Circle, CirclePathIter, Point, Rect, Shape, Vec2};
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq)]
pub struct ShapeDisc {
    center: Position,
    radius: Value,

    highlighted: bool,
    selected: bool,

    segs: BezPath,
    polygon: Polygon<f64>,
}
impl ShapeDisc {
    const MIN_RADIUS: f64 = 2.;

    pub fn new(center: Vec2, _pos2: Vec2) -> BSKind {
        let center = Position::new(center, false);
        let mut radius = Value::new(ShapeDisc::MIN_RADIUS);
        radius.select(true);

        BSKind::Disc(ShapeDisc {
            center,
            radius,
            highlighted: false,
            selected: false,
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }
    pub fn get_polygon(&self) -> Polygon<f64> {
        self.polygon.clone()
    }

    fn update_polygon(&mut self) {
        self.segs = calc_segs(self.get_paths());
        self.polygon = calc_polygon(&self.segs);
    }

    fn get_circle(&self) -> Circle {
        let center = self.center.get_pos();
        let radius = self.radius.get_val();
        Circle::new(center.to_point(), radius)
    }
    fn get_radius_modifier(&self) -> Vec2 {
        let center = self.center.get_pos();
        let radius = self.radius.get_val();
        center + Vec2::new(radius, 0.)
    }
}
impl Display for ShapeDisc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Circle")
    }
}
impl Shape for ShapeDisc {
    type PathElementsIter<'iter> = CirclePathIter;

    fn path_elements(&self, tolerance: f64) -> CirclePathIter {
        self.get_circle().path_elements(tolerance)
    }
    #[inline]
    fn area(&self) -> f64 {
        self.get_circle().area()
    }
    #[inline]
    fn perimeter(&self, accuracy: f64) -> f64 {
        self.get_circle().perimeter(accuracy)
    }
    #[inline]
    fn winding(&self, pt: Point) -> i32 {
        self.get_circle().winding(pt)
    }
    #[inline]
    fn bounding_box(&self) -> Rect {
        self.get_circle().bounding_box()
    }
    #[inline]
    fn as_circle(&self) -> Option<Circle> {
        self.get_circle().as_circle()
    }
    #[inline]
    fn contains(&self, pt: Point) -> bool {
        self.get_circle().contains(pt)
    }
}
impl ObjectsFuncs for ShapeDisc {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 2.;
    type Kindvars = BSKindvars;

    fn save_vars(&mut self) {
        self.center.save_pos();
        self.radius.save_val();
    }
    fn restore_saved(&mut self) {
        self.center.restore_saved();
        self.radius.restore_saved();
        self.update_polygon();
    }
    fn get_vars(&self) -> BSKindvars {
        BSKindvars::Disc(self.center, self.radius)
    }
    fn set_vars(&mut self, vars: &BSKindvars) {
        if let BSKindvars::Disc(center, radius) = vars {
            self.center = center.clone();
            self.radius = radius.clone();
        }
        self.update_polygon();
    }
    fn good_size(&self) -> bool {
        self.radius.get_val() >= ShapeDisc::MIN_RADIUS
    }

    fn set_hs_from_pos(&mut self, pos: Vec2, hors: HS) -> bool {
        match hors {
            HS::Highlight => {
                self.highlighted = self.contains(pos.to_point());
                self.highlighted
            }
            HS::Select => {
                self.selected = self.contains(pos.to_point());
                self.selected
            }
        }
    }
    fn set_hs(&mut self, value: bool, hors: HS) {
        match hors {
            HS::Highlight => self.highlighted = value,
            HS::Select => self.selected = value,
        }
    }
    fn get_hs(&self, hors: HS) -> bool {
        match hors {
            HS::Highlight => self.highlighted,
            HS::Select => self.selected,
        }
    }
    fn get_hhss(&self) -> (bool, bool) {
        (self.selected, self.highlighted)
    }

    fn set_hs_modifiers_from_pos(&mut self, pos: Vec2, hors: HS) -> bool {
        let radius_hors = (pos - (self.get_radius_modifier())).hypot() < Self::GRAB;
        match hors {
            HS::Highlight => {
                self.radius.highlight(radius_hors);
                self.radius.is_highlighted()
            }
            HS::Select => {
                self.radius.select(radius_hors);
                self.radius.is_selected()
            }
        }
    }
    fn set_hs_modifiers(&mut self, value: bool, hors: HS) {
        match hors {
            HS::Highlight => {
                self.radius.highlight(value);
            }
            HS::Select => {
                self.radius.select(value);
            }
        }
    }
    fn get_hs_modifiers(&self, hors: HS) -> bool {
        match hors {
            HS::Highlight => self.radius.is_highlighted(),
            HS::Select => self.radius.is_selected(),
        }
    }

    fn toggle_prop(&mut self) {
        ()
    }

    fn move_position(&mut self, dpos: Vec2) {
        self.center.set_pos(self.center.get_saved_pos() + dpos);
        self.update_polygon();
    }
    fn move_modifier(&mut self, pos_init: Vec2, pos: Vec2, _shift_pressed: bool) -> bool {
        let dpos = pos - pos_init;
        let saved_radius = self.radius.get_saved_val();
        let radius = saved_radius + dpos.x;
        if radius >= ShapeDisc::MIN_RADIUS {
            self.radius.set_val(radius);
            self.update_polygon();
            true
        } else {
            false
        }
    }
    fn get_position(&self) -> Vec2 {
        self.center.get_pos()
    }

    fn get_modifiers_paths(&self) -> Vec<(BezPath, Pattern)> {
        vec![
            (
                modifiers_path(self.get_radius_modifier(), 1., ShapeDisc::GRAB),
                self.get_pattern_modifiers(self.radius.is_selected(), self.radius.is_highlighted()),
            ),
            (
                center_path(self.center.get_pos(), 1., ShapeDisc::GRAB),
                self.get_pattern_modifiers(self.selected, self.highlighted),
            ),
        ]
    }
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        let mut paths = vec![];
        let mut texts = vec![];
        let offset = self.radius.get_val() / 2_f64.sqrt();
        let end = self.center.get_pos() + Vec2::new(offset, -offset);
        let (path, text) = Dimension::new(DimKind::Radius, self.center.get_pos(), end).get_path();
        paths.push(path);
        texts.push(text);
        (paths, texts)
    }
    fn get_paths(&self) -> Vec<BezPath> {
        vec![self.get_circle().to_path(Self::TOLERANCE)]
    }
}
