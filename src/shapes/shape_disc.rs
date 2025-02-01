// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }

use crate::{
    canvas::{CanvasText, Pattern},
    dimensions::{DimKind, Dimension},
    math::*,
    pools::HS,
    positions::{Position, Value},
    prefab::{center_path, modifiers_pattern},
    traits::*,
    KeysStates, Pointer,
};
use geo::{LineString, Polygon};
use kurbo::{BezPath, Circle, CirclePathIter, Point, Rect, Shape, Size, Vec2};
use std::fmt::Display;

use super::shapes::ShapeKind;

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

    pub fn new(center: Vec2, pos2: Vec2) -> ShapeKind {
        use HS::*;
        let center = Position::new(center, false);
        let mut radius = Value::new((pos2 - center.pos).hypot());
        radius.set_hs(Select, true);

        ShapeKind::KindDisc(ShapeDisc {
            center,
            radius,
            highlighted: false,
            selected: false,
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }
    fn get_circle(&self) -> Circle {
        let center = self.center.pos;
        let radius = self.radius.value;
        Circle::new(center.to_point(), radius)
    }
    fn hs_controls_from_pos(&mut self, pointer: &mut Pointer, _keys_states: KeysStates, hs: HS) {
        use HS::*;

        // circonference
        let within_grab_radius = |pos: Vec2, center: Vec2, radius: f64| -> bool {
            ((pos - center).hypot() - radius).abs() < Self::GRAB_RADIUS
        };
        self.radius.set_hs(
            hs,
            within_grab_radius(pointer.pos(), self.center.pos, self.radius.value),
        );

        // Pointer update
        if self.radius.is_hs(hs) {
            pointer.set_pos(
                self.center.pos + (pointer.pos() - self.center.pos).normalize() * self.radius.value,
            );
        }
        if self.radius.is_hs(Select) {
            pointer.save_pos();
        }
    }

    pub fn get_magnet_points(&self) -> Vec<Vec2> {
        vec![self.center.pos]
    }
    pub fn get_polygon(&self) -> Polygon<f64> {
        self.polygon.clone()
    }
    fn update_polygon(&mut self) {
        self.segs = calc_segs(self.to_path(Self::TOLERANCE));
        self.polygon = calc_polygon(&self.segs);
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
    type Kindvars = ShapeKind;

    fn save_vars(&mut self) {
        self.center.saved_pos = self.center.pos;
        self.radius.saved_val = self.radius.value;
    }
    fn restore_vars(&mut self) {
        self.center.pos = self.center.saved_pos;
        self.radius.value = self.radius.saved_val;
        self.update_polygon();
    }
    fn get_vars(&self) -> ShapeKind {
        ShapeKind::KindDisc(ShapeDisc {
            center: self.center.clone(),
            radius: self.radius.clone(),
            highlighted: false,
            selected: false,
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }
    fn set_vars(&mut self, vars: &ShapeKind) {
        if let ShapeKind::KindDisc(shape_disc) = vars {
            self.center = shape_disc.center;
            self.radius = shape_disc.radius;
        }
        self.update_polygon();
    }
    fn good_size(&self) -> bool {
        self.radius.value >= ShapeDisc::MIN_RADIUS
    }
    fn finish_draw(&mut self) -> bool {
        self.radius.value >= ShapeDisc::MIN_RADIUS
    }
    fn get_state(&self, get: GetEntityState) -> Option<Vec2> {
        use GetEntityState::*;
        use HS::*;
        match get {
            IsHS(Select) => self.selected.then(|| self.get_position()),
            IsHS(Highlight) => self.highlighted.then(|| self.get_position()),
            GetFirstControlHS(Select) => self.radius.is_hs(Select).then(|| self.get_position()),
            GetFirstControlHS(Highlight) => {
                self.radius.is_hs(Highlight).then(|| self.get_position())
            }
        }
    }
    fn set_state(&mut self, set: SetEntityState) {
        use SetEntityState::*;
        use HS::*;
        match set {
            SetHS(Select, value) => self.selected = value,
            SetHS(Highlight, value) => self.highlighted = value,
            SetAllControlsHS(hs, value) => {
                self.center.set_hs(hs, value);
                self.radius.set_hs(hs, value);
            }
        }
    }
    fn set_state_from_pos(
        &mut self,
        pointer: &mut Pointer,
        keys_states: KeysStates,
        set: SetEntityStateFromPos,
    ) {
        use SetEntityStateFromPos::*;
        match set {
            SetHSFromPos(hs) => match hs {
                HS::Select => {
                    self.selected = self.contains(pointer.pos().to_point());
                    if self.selected {
                        pointer.set_pos(self.center.pos);
                        pointer.save_pos();
                    }
                }
                HS::Highlight => {
                    self.highlighted = self.contains(pointer.pos().to_point());
                    // pointer.set_pos(self.center.pos);
                }
            },
            SetControlHSFromPos(hs) => self.hs_controls_from_pos(pointer, keys_states, hs),
        }
    }

    fn toggle_selected_prop(&mut self) {
        ()
    }

    fn move_position(&mut self, pointer: &mut Pointer, _keys_states: KeysStates) -> bool {
        let dpos = pointer.dpos();
        self.center.pos = self.center.saved_pos + dpos;
        self.update_polygon();
        true
    }
    fn move_controls(&mut self, pointer: &Pointer, _keys_states: KeysStates) -> bool {
        let radius = if pointer.is_magnetized() {
            (pointer.pos() - self.center.pos).hypot()
        } else {
            snap_val(
                (pointer.pos() - self.center.pos).hypot(),
                pointer.get_snap().val(),
            )
        };
        if radius >= ShapeDisc::MIN_RADIUS {
            self.radius.value = radius;
            self.update_polygon();
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
        vec![(
            center_path(self.center.pos, 1., ShapeDisc::GRAB_RADIUS),
            modifiers_pattern(self.selected, self.highlighted),
        )]
    }
    fn get_dimensions_paths_and_patterns(
        &self,
        _: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        let mut res = vec![];
        let offset = self.radius.value / 2_f64.sqrt();
        let end = self.center.pos + Vec2::new(offset, -offset);
        let dim = Dimension::new(DimKind::Radius, self.center.pos, end, self.radius.value)
            .get_path_and_pattern();
        res.push(dim);
        res
    }
    fn get_paths_and_patterns(&self, _: &Size, _: (Rect, f64, Vec2)) -> Vec<(BezPath, Pattern)> {
        use HS::*;
        let pattern = if self.selected {
            Pattern::BasicSelected
        } else if self.highlighted {
            Pattern::BasicHighlighted
        } else {
            if self.radius.is_hs(Select) {
                Pattern::BasicLightSelected
            } else if self.radius.is_hs(Highlight) {
                Pattern::BasicLightHighlighted
            } else {
                Pattern::BasicNormal
            }
        };
        vec![(self.to_path(Self::TOLERANCE), pattern)]
    }
}
