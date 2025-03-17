// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }

use crate::{
    canvas::{CanvasText, Pattern},
    math::*,
    pools::HS,
    positions::{Position, Status, Value},
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
    radius_state: Status,
    state: Status,

    segs: BezPath,
    polygon: Polygon<f64>,
}
impl ShapeDisc {
    const MIN_RADIUS: f64 = 2.;

    pub fn new(center: Vec2, pos2: Vec2) -> Option<ShapeKind> {
        let center = Position::new(center);
        let radius = Value::new((pos2 - center.pos).hypot());
        if radius.value < EPSILON {
            return None;
        }
        let radius_state = Status::default();
        let mut shape_disc = ShapeDisc {
            center,
            radius,
            radius_state,
            state: Status::default(),
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        };
        shape_disc.update_geo_polygon();
        Some(ShapeKind::KindDisc(shape_disc))
    }
    fn get_circle(&self) -> Circle {
        let center = self.center.pos;
        let radius = self.radius.value;
        Circle::new(center.to_point(), radius)
    }
    pub fn get_polygon(&self) -> Polygon<f64> {
        self.polygon.clone()
    }
    pub fn update_geo_polygon(&mut self) {
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
        self.update_geo_polygon();
    }
    fn get_vars(&self) -> ShapeKind {
        ShapeKind::KindDisc(ShapeDisc {
            center: self.center.clone(),
            radius: self.radius.clone(),
            radius_state: Status::default(),
            state: Status::default(),
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }
    fn set_vars(&mut self, vars: &ShapeKind) {
        if let ShapeKind::KindDisc(shape_disc) = vars {
            self.center = shape_disc.center;
            self.radius = shape_disc.radius;
        }
        self.update_geo_polygon();
    }

    fn get_state(&self, get: GetEntityState) -> bool {
        use GetEntityState::*;
        match get {
            IsHS(hs) => self.state.is_hs(hs),
            IsAControlHS(hs) => self.radius_state.is_hs(hs),
        }
    }
    fn set_state(&mut self, set: SetEntityState) {
        use SetEntityState::*;
        match set {
            SetHS(hs, value) => self.state.set_hs(hs, value),
            SetAllControlsHS(hs, value) => self.radius_state.set_hs(hs, value),
        }
    }
    fn set_state_from_pos(
        &mut self,
        pointer: &mut Pointer,
        keys_states: KeysStates,
        set: SetEntityStateFromPos,
    ) -> bool {
        use SetEntityStateFromPos::*;
        match set {
            SetHSFromPos(hs) => {
                let state = self.contains(pointer.pos().to_point());
                self.state.set_hs(hs, state);
                state
            }
            SetControlHSFromPos(hs) => {
                use HS::*;
                self.state.set_hs(hs, false);
                self.radius_state.set_hs(hs, false);
                // circonference
                if ((pointer.pos() - self.center.pos).hypot() - self.radius.value).abs()
                    < Self::GRAB_RADIUS
                {
                    self.radius_state.set_hs(hs, true);
                    if !keys_states.alt_pressed {
                        pointer.set_pos(
                            self.center.pos
                                + (pointer.pos() - self.center.pos).normalize() * self.radius.value,
                        );
                        if self.radius_state.is_hs(Select) {
                            pointer.save_pos();
                        }
                        pointer.set_magnetized(true);
                    }
                    true
                } else {
                    false
                }
            }
        }
    }
    fn contains_pointer(&self, pointer: &Pointer) -> bool {
        self.contains(pointer.pos().to_point())
    }
    fn move_position(&mut self, pointer: &mut Pointer, _keys_states: KeysStates) -> bool {
        let dpos = pointer.dpos();
        self.center.pos = self.center.saved_pos + dpos;
        self.update_geo_polygon();
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
            self.update_geo_polygon();
            return true;
        };
        false
    }

    fn get_position(&self) -> Vec2 {
        self.center.pos
    }

    fn get_controls_paths_and_patterns(
        &self,
        _: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        use HS::*;
        vec![(
            center_path(self.center.pos, 1., ShapeDisc::GRAB_RADIUS),
            modifiers_pattern(self.state.is_hs(Select), self.state.is_hs(Highlight)),
        )]
    }
    fn get_dimensions_paths_and_patterns(
        &self,
        _: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        let mut res = vec![];
        let r = self.radius.value / 2_f64.sqrt();
        let end = Vec2::new(r, r) + self.center.pos;
        let start = self.center.pos;
        // Dimension::new(DimKind::Linear, end, start, cinfo).and_then(|dim| {
        //     res.push(dim.get_path_and_pattern());
        //     Some(())
        // });
        res
    }
    fn get_paths_and_patterns(&self, _: &Size, _: (Rect, f64, Vec2)) -> Vec<(BezPath, Pattern)> {
        use HS::*;
        let pattern = if self.state.is_hs(Select) {
            Pattern::BasicSelected
        } else if self.state.is_hs(Highlight) {
            Pattern::BasicHighlighted
        } else {
            if self.radius_state.is_hs(Select) {
                Pattern::BasicLightSelected
            } else if self.radius_state.is_hs(Highlight) {
                Pattern::BasicLightHighlighted
            } else {
                Pattern::BasicNormal
            }
        };
        vec![(self.to_path(Self::TOLERANCE), pattern)]
    }
    fn get_prim_paths_and_patterns(
        &self,
        _: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        vec![]
    }
}
