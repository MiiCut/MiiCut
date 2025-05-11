use super::shapes::ShapeKind;
use crate::{
    canvas::{CanvasText, Colors, Pattern},
    dimensions::dim_radius,
    math::*,
    pools::HS,
    positions::Status,
    prefab::{centroid_path, get_centroids_colors, get_shapes_colors},
    traits::*,
    KeysStates, Pointer,
};
use geo::{LineString, Polygon};
use kurbo::{BezPath, Circle, CirclePathIter, Point, Rect, Shape, Size, Vec2};
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq)]
pub struct ShapeDisc {
    bdl: SegBundle,
    bdl_saved: SegBundle,
    radius_state: Status,
    state: Status,

    segs: BezPath,
    polygon: Polygon<f64>,
}
impl ShapeDisc {
    const MIN_RADIUS: f64 = 2.;

    pub fn new(center: Vec2, pos2: Vec2) -> Option<ShapeKind> {
        (center - pos2).hypot().gt(&Self::MIN_RADIUS).then(|| {
            SegBundle::new(center, pos2).and_then(|bdl| {
                let mut shape_disc = ShapeDisc {
                    bdl,
                    bdl_saved: bdl,
                    radius_state: Status::default(),
                    state: Status::default(),
                    segs: BezPath::new(),
                    polygon: Polygon::new(LineString::new(vec![]), vec![]),
                };
                shape_disc.update_geo_polygon();
                Some(ShapeKind::KindDisc(shape_disc))
            })
        })?
    }
    fn get_circle(&self) -> Circle {
        let center = self.bdl.s();
        let radius = self.bdl.len();
        Circle::new(center.to_point(), radius)
    }
    pub fn get_polygon(&self) -> Polygon<f64> {
        self.polygon.clone()
    }
    pub fn update_geo_polygon(&mut self) {
        self.segs = calc_segs(self.to_path(Self::TOLERANCE));
        self.polygon = calc_polygon(&self.segs);
    }
    pub fn get_seg_bdl(&self) -> &SegBundle {
        &self.bdl
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
    const GRAB_RADIUS: f64 = 10.;
    type Kindvars = ShapeKind;

    fn tab(&mut self) -> bool {
        false
    }
    fn save_vars(&mut self) {
        self.bdl_saved = self.bdl;
    }
    fn restore_vars(&mut self) {
        self.bdl = self.bdl_saved;
    }
    fn get_vars(&self) -> ShapeKind {
        ShapeKind::KindDisc(ShapeDisc {
            bdl: self.bdl.clone(),
            bdl_saved: self.bdl_saved.clone(),
            radius_state: self.radius_state.clone(),
            state: Status::default(),
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }
    fn set_vars(&mut self, shape_kind: &ShapeKind) {
        if let ShapeKind::KindDisc(shape_disc) = shape_kind {
            self.bdl = shape_disc.bdl.clone();
            self.bdl_saved = shape_disc.bdl_saved.clone();
            self.radius_state = shape_disc.radius_state.clone();
            self.state = Status::default();
            self.segs = BezPath::new();
            self.polygon = Polygon::new(LineString::new(vec![]), vec![]);
        }
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
                if ((pointer.pos() - self.bdl.s()).hypot() - self.bdl.len()).abs()
                    < Self::GRAB_RADIUS / pointer.get_draw_scale()
                {
                    self.radius_state.set_hs(hs, true);
                    if !keys_states.alt_pressed {
                        pointer.set_pos(
                            self.bdl.s()
                                + (pointer.pos() - self.bdl.s()).normalize() * self.bdl.len(),
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
        let mut set = self.bdl.try_set_s(self.bdl_saved.s() + pointer.dpos());
        set |= self.bdl.try_set_e(self.bdl_saved.e() + pointer.dpos());
        self.update_geo_polygon();
        set
    }
    fn move_controls(&mut self, pointer: &Pointer, _keys_states: KeysStates) -> bool {
        use HS::*;
        if self.radius_state.is_hs(Select) {
            let radius_pt = if !pointer.is_magnetized() {
                snap_length(self.bdl.s(), pointer.pos(), pointer.get_snap().val())
            } else {
                pointer.pos()
            };
            if (radius_pt - self.bdl.s()).hypot() >= ShapeDisc::MIN_RADIUS {
                self.bdl.try_set_e(radius_pt);
            }
            self.update_geo_polygon();
            true
        } else {
            false
        }
    }
    fn get_position(&self) -> Vec2 {
        self.bdl.s()
    }
    fn get_centroid(&self) -> Vec<Vec2> {
        vec![self.bdl.s()]
    }
    fn get_controls_paths_and_patterns(
        &self,
        _: &Size,
        canvas_infos: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, Colors)> {
        let scale = canvas_infos.1;
        vec![(
            centroid_path(self.get_centroid()[0], scale, Self::GRAB_RADIUS),
            Pattern::Point,
            get_centroids_colors(self.state),
        )]
    }
    fn get_dimensions_paths_and_patterns(
        &self,
        _: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, Colors, Vec<CanvasText>)> {
        vec![dim_radius(self.bdl, cinfo, self.radius_state)]
    }
    fn get_paths_and_patterns(
        &self,
        _: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, Colors)> {
        vec![(
            self.to_path(Self::TOLERANCE),
            Pattern::Basic,
            get_shapes_colors(self.state),
        )]
    }
    fn get_prim_paths_and_patterns(
        &self,
        _: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, Colors)> {
        vec![]
    }
}
