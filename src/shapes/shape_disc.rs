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
    Pointer,
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
            IsHighligh => {
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
            IsAnyModifierHighligh => {
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
            SetHighli(value) => self.highlighted = value,
            SelectAllModifiers(value) => self.select_all_modifiers(value),
            HighliAllModifiers(value) => self.highlight_all_modifiers(value),
        }
    }
    fn set_state_from_pos(&mut self, pointer: &mut Pointer, set: SetEntityStateFromPos) {
        use SetEntityStateFromPos::*;
        match set {
            SelectFromPos => {
                self.selected = self.contains(pointer.pos().to_point());
            }
            HighliFromPos => {
                self.highlighted = self.contains(pointer.pos().to_point());
            }
            SelectModifierFromPos => {
                self.select_modifiers_from_pos(pointer.pos(), Self::GRAB_RADIUS);
            }
            HighliModifierFromPos => {
                self.highlight_modifiers_from_pos(pointer.pos(), Self::GRAB_RADIUS);
            }
        }
    }

    fn toggle_prop(&mut self) {
        ()
    }

    fn move_position(&mut self, pointer: &mut Pointer, _shift_pressed: bool) -> bool {
        let dpos = pointer.dpos();
        self.center.pos = self.center.saved_pos + dpos;
        self.update_polygon();
        true
    }
    fn move_modifier(&mut self, pointer: &mut Pointer, _shift_pressed: bool) -> bool {
        let saved_radius = self.radius.saved_val;
        let radius = snap_val(saved_radius + pointer.dpos().x, pointer.get_snap().val());
        if radius >= ShapeDisc::MIN_RADIUS {
            self.radius.value = radius;
            self.update_polygon();
            pointer.set_pos(self.get_radius_modifier());
            return true;
        };
        false
    }
    fn get_position(&self) -> Vec2 {
        self.center.pos
    }

    fn get_mod_paths_and_patterns(
        &self,
        _: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        vec![
            (
                modifiers_path(self.get_radius_modifier(), 1., ShapeDisc::GRAB_RADIUS),
                self.get_pattern_status(self.radius.selected, self.radius.highlighted),
            ),
            (
                center_path(self.center.pos, 1., ShapeDisc::GRAB_RADIUS),
                self.get_pattern_status(self.selected, self.highlighted),
            ),
        ]
    }
    fn get_dimensions_paths_and_patterns(
        &self,
        _: &Size,
        _: (Rect, f64, Vec2),
    ) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        let mut paths = vec![];
        let mut texts = vec![];
        let offset = self.radius.value / 2_f64.sqrt();
        let end = self.center.pos + Vec2::new(offset, -offset);
        let (path, text) = Dimension::new(DimKind::Radius, self.center.pos, end, self.radius.value)
            .get_path_and_pattern();
        paths.push(path);
        texts.push(text);
        (paths, texts)
    }
    fn get_paths(&self, _: &Size) -> Vec<BezPath> {
        vec![self.get_circle().to_path(Self::TOLERANCE)]
    }
    fn get_paths_and_patterns(&self, das: &Size, _: (Rect, f64, Vec2)) -> Vec<(BezPath, Pattern)> {
        let pattern = match (self.selected, self.highlighted) {
            (false, false) => Pattern::BasicNormal,
            (false, true) => Pattern::BasicHighlighted,
            (true, false) => Pattern::BasicSelected,
            (true, true) => Pattern::BasicSelected,
        };

        let mut paths = self.get_paths(das);
        let result = paths
            .iter_mut()
            .map(|path| (path.clone(), pattern))
            .collect();
        result
    }
}
