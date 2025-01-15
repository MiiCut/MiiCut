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
    positions::{Position, Value},
    prefab::{center_path, modifiers_path},
    traits::*,
};
use geo::{LineString, Polygon};
use kurbo::{BezPath, Circle, CirclePathIter, Point, Rect, Shape, Size, Vec2};
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
        let mut radius = Value::new(0.);
        radius.selected = true;

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
        self.segs = calc_segs(self.get_paths(&Size::ZERO));
        self.polygon = calc_polygon(&self.segs);
    }

    fn get_circle(&self) -> Circle {
        let center = self.center.pos;
        let radius = self.radius.value;
        Circle::new(center.to_point(), radius)
    }
    fn get_radius_modifier(&self) -> Vec2 {
        let center = self.center.pos;
        let radius = self.radius.value;
        center + Vec2::new(radius, 0.)
    }

    fn highlight_all_modifiers(&mut self, value: bool) {
        self.radius.highlighted = value;
    }
    fn select_all_modifiers(&mut self, value: bool) {
        self.radius.selected = value;
    }

    fn highlight_modifiers_from_pos(&mut self, pos: Vec2, grab: f64) {
        self.radius.highlighted = (pos - self.get_radius_modifier()).hypot() < grab;
    }
    fn select_modifiers_from_pos(&mut self, pos: Vec2, grab: f64) {
        self.radius.selected = (pos - self.get_radius_modifier()).hypot() < grab;
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
    const GRAB_RADIUS: f64 = 5.;
    type Kindvars = BSKindvars;

    fn save_vars(&mut self) {
        self.center.saved_pos = self.center.pos;
        self.radius.saved_val = self.radius.value;
    }
    fn restore_saved(&mut self) {
        self.center.pos = self.center.saved_pos;
        self.radius.value = self.radius.saved_val;
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
        self.radius.value >= ShapeDisc::MIN_RADIUS
    }

    fn get_state(&self, get: GetEntityState) -> Option<Vec2> {
        use GetEntityState::*;
        match get {
            IsSelected => {
                if self.selected {
                    Some(self.get_position())
                } else {
                    None
                }
            }
            IsHighlighted => {
                if self.highlighted {
                    Some(self.get_position())
                } else {
                    None
                }
            }
            IsAnyModifierSelected => {
                let select = self.radius.selected;
                if select {
                    Some(self.get_position())
                } else {
                    None
                }
            }
            IsAnyModifierHighlighted => {
                let highlight = self.radius.highlighted;
                if highlight {
                    Some(self.get_position())
                } else {
                    None
                }
            }
        }
    }
    fn set_state(&mut self, set: SetEntityState) {
        use SetEntityState::*;
        match set {
            SetSelect(value) => self.selected = value,
            SelectFromPos(pos, ..) => {
                self.selected = self.contains(pos.to_point());
            }
            SetHighlight(value) => self.highlighted = value,
            HighlightFromPos(pos, ..) => {
                self.highlighted = self.contains(pos.to_point());
            }

            SelectAllModifiers(value) => self.select_all_modifiers(value),
            SelectModifierFromPos(pos, precision, _) => {
                self.select_modifiers_from_pos(pos, precision);
            }

            HighlightAllModifiers(value) => self.highlight_all_modifiers(value),
            HighlightModifierFromPos(pos, precision, _) => {
                self.highlight_modifiers_from_pos(pos, precision);
            }
        }
    }

    fn toggle_prop(&mut self) {
        ()
    }

    fn move_position(&mut self, dpos: Vec2, snap: f64) -> Option<Vec2> {
        self.center.pos = snap_pt(self.center.saved_pos + dpos, snap);
        self.update_polygon();
        Some(self.get_position())
    }
    fn move_modifier(
        &mut self,
        pos_init: Vec2,
        pos: Vec2,
        snap: f64,
        _shift_pressed: bool,
    ) -> Option<Vec2> {
        let dpos = pos - pos_init;
        let saved_radius = self.radius.saved_val;
        let radius = snap_val(saved_radius + dpos.x, snap);
        if radius >= ShapeDisc::MIN_RADIUS {
            self.radius.value = radius;
            self.update_polygon();
            Some(self.get_radius_modifier())
        } else {
            None
        }
    }
    fn get_position(&self) -> Vec2 {
        self.center.pos
    }

    fn get_modifiers_paths(&self, _: &Size) -> Vec<(BezPath, Pattern)> {
        vec![
            (
                modifiers_path(self.get_radius_modifier(), 1., ShapeDisc::GRAB_RADIUS),
                self.get_pattern_modifiers(self.radius.selected, self.radius.highlighted),
            ),
            (
                center_path(self.center.pos, 1., ShapeDisc::GRAB_RADIUS),
                self.get_pattern_modifiers(self.selected, self.highlighted),
            ),
        ]
    }
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        let mut paths = vec![];
        let mut texts = vec![];
        let offset = self.radius.value / 2_f64.sqrt();
        let end = self.center.pos + Vec2::new(offset, -offset);
        let (path, text) =
            Dimension::new(DimKind::Radius, self.center.pos, end, self.radius.value).get_path();
        paths.push(path);
        texts.push(text);
        (paths, texts)
    }
    fn get_paths(&self, _: &Size) -> Vec<BezPath> {
        vec![self.get_circle().to_path(Self::TOLERANCE)]
    }
    fn get_paths_and_patterns(&self, drawing_area_size: &Size) -> Vec<(BezPath, Pattern)> {
        let pattern = match (self.selected, self.highlighted) {
            (false, false) => Pattern::BasicNormal,
            (false, true) => Pattern::BasicHighlighted,
            (true, false) => Pattern::BasicSelected,
            (true, true) => Pattern::BasicSelected,
        };

        let mut paths = self.get_paths(drawing_area_size);
        let result = paths
            .iter_mut()
            .map(|path| (path.clone(), pattern))
            .collect();
        result
    }
}
