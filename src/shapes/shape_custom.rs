// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
use super::shapes::ShapeKind;
use crate::{
    canvas::{CanvasText, Pattern},
    math::*,
    pools::HS,
    positions::Position,
    prefab::*,
    primitives::primitives::{
        GetPrimitiveState, Primitive, PrimitiveControls, PrimitiveCurve, SetPrimitiveState,
        SetPrimitiveStateFromPos, VertexModifier, VertexProperty,
    },
    traits::*,
    KeysStates, Pointer,
};
use geo::{LineString, Polygon};
use kurbo::{BezPath, Line, PathEl, Point, Rect, Shape, Size, Vec2};
use std::{f64::consts::PI, fmt::Display};

#[derive(Clone, Debug)]
pub struct ShapeCustom {
    prims: Vec<Primitive>,
    current_creation_pos: Option<Position>,

    primitivess_start_property: VertexProperty,

    highlighted: bool,
    selected: bool,

    segs: BezPath,
    polygon: Polygon<f64>,
}
impl ShapeCustom {
    pub const MIN_RECT_SIZE: f64 = 10.;

    pub fn new(prims_property: VertexProperty, pos1: Vec2, pos2: Vec2) -> ShapeKind {
        match prims_property {
            VertexProperty::Nope => ShapeKind::KindPolygon(ShapeCustom {
                prims: vec![Primitive::new_old(
                    PrimitiveCurve::CurveLine,
                    VertexProperty::Nope,
                    pos1,
                    pos2,
                )],
                current_creation_pos: Some(Position::new(pos2, true)),
                primitivess_start_property: VertexProperty::Nope,
                highlighted: false,
                selected: false,
                segs: BezPath::new(),
                polygon: Polygon::new(LineString::new(vec![]), vec![]),
            }),
            VertexProperty::RectangleLike => {
                let p1 = Primitive::new_old(
                    PrimitiveCurve::CurveLine,
                    VertexProperty::RectangleLike,
                    pos1,
                    Vec2::new(pos2.x, pos1.y),
                );
                let p2 = Primitive::new_old(
                    PrimitiveCurve::CurveLine,
                    VertexProperty::RectangleLike,
                    Vec2::new(pos2.x, pos1.y),
                    pos2,
                );
                let mut p3 = Primitive::new_old(
                    PrimitiveCurve::CurveLine,
                    VertexProperty::RectangleLike,
                    pos2,
                    Vec2::new(pos1.x, pos2.y),
                );
                p3.set_state(SetPrimitiveState::SetStartHS(HS::Select, true));
                let p4 = Primitive::new_old(
                    PrimitiveCurve::CurveLine,
                    VertexProperty::RectangleLike,
                    Vec2::new(pos1.x, pos2.y),
                    pos1,
                );

                ShapeKind::KindPolygon(ShapeCustom {
                    prims: vec![p1, p2, p3, p4],
                    current_creation_pos: Some(Position::new(Vec2::new(pos1.x, pos2.y), true)),
                    primitivess_start_property: VertexProperty::RectangleLike,
                    highlighted: false,
                    selected: false,
                    segs: BezPath::new(),
                    polygon: Polygon::new(LineString::new(vec![]), vec![]),
                })
            }
        }
    }
    pub fn add_point(&mut self, pointer: &mut Pointer) {
        let pos = pointer.pos();
        // Get the last line drawn
        if let Some(last_line) = self.prims.last_mut() {
            if let PrimitiveCurve::CurveLine = last_line.get_prim_curve() {
                if let Some(current_pos) = &mut self.current_creation_pos {
                    current_pos.pos = pos;
                    current_pos.saved_pos = pos;
                    last_line.set_end_pos(pos);
                    self.prims.push(Primitive::new_old(
                        PrimitiveCurve::CurveLine,
                        self.primitivess_start_property,
                        pos,
                        pos,
                    ));
                    self.update_polygon();
                }
            }
        }
    }
    pub fn end_creation(&mut self) -> bool {
        if self.good_size() {
            self.current_creation_pos = None;
            let first_pos = self.prims.first().unwrap().get_start_pos();
            if let Some(last_prim) = self.prims.last_mut() {
                if let PrimitiveCurve::CurveLine = last_prim.get_prim_curve() {
                    last_prim.set_end_pos(first_pos);
                }
            }
            self.update_polygon();
            true
        } else {
            false
        }
    }
    pub fn get_polygon(&self) -> Polygon<f64> {
        self.polygon.clone()
    }
    pub fn update_polygon(&mut self) {
        self.segs = calc_segs(self.to_path(Self::TOLERANCE));
        self.polygon = calc_polygon(&self.segs);
    }
    pub fn get_width(&self) -> f64 {
        let rect = self.get_bounding_box();
        rect.width()
    }
    pub fn get_height(&self) -> f64 {
        let rect = self.get_bounding_box();
        rect.height()
    }
    pub fn get_bounding_box(&self) -> Rect {
        if self.prims.is_empty() {
            return Rect::new(0., 0., 0., 0.);
        }

        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for prim in self.prims.iter() {
            let start = prim.get_start_pos();
            if start.x < min_x {
                min_x = start.x;
            }
            if start.y < min_y {
                min_y = start.y;
            }
            if start.x > max_x {
                max_x = start.x;
            }
            if start.y > max_y {
                max_y = start.y;
            }
        }
        Rect::new(min_x, min_y, max_x, max_y)
    }
    pub fn get_primitivess_start_property(&self) -> VertexProperty {
        self.primitivess_start_property
    }
    pub fn get_prims(&self) -> &Vec<Primitive> {
        &self.prims
    }
    pub fn get_prims_mut(&mut self) -> &mut Vec<Primitive> {
        &mut self.prims
    }

    fn get_vertices_centroid(&self) -> Vec2 {
        let mut centroid = Vec2::ZERO;
        self.prims.iter().for_each(|prim| {
            centroid += prim.get_start_pos();
        });
        centroid / self.prims.len() as f64
    }
    fn line_to(&self, start: Vec2, end: Vec2) -> BezPath {
        Line::new(start.to_point(), end.to_point()).into_path(Self::TOLERANCE)
    }
    fn get_prim(&self, idx: usize) -> &Primitive {
        &self.prims[idx % self.prims.len()]
    }
    fn get_prev_prim(&self, idx: usize) -> &Primitive {
        let prev_index = if idx == 0 {
            self.prims.len() - 1
        } else {
            idx - 1
        };
        &self.prims[prev_index]
    }
    fn get_next_prim(&self, idx: usize) -> &Primitive {
        let next_index = (idx + 1) % self.prims.len();
        &self.prims[next_index]
    }
    fn get_prim_mut(&mut self, idx: usize) -> &mut Primitive {
        let len = self.prims.len();
        &mut self.prims[idx % len]
    }
    fn get_prev_prim_mut(&mut self, idx: usize) -> &mut Primitive {
        let prev_index = if idx == 0 {
            self.prims.len() - 1
        } else {
            idx - 1
        };
        &mut self.prims[prev_index]
    }
    fn get_prev_prev_prim_mut(&mut self, idx: usize) -> &mut Primitive {
        let prev_index = if idx == 0 {
            self.prims.len() - 1
        } else {
            idx - 1
        };
        let prev_prev_index = if prev_index == 0 {
            self.prims.len() - 1
        } else {
            prev_index - 1
        };
        &mut self.prims[prev_prev_index]
    }
    fn get_next_prim_mut(&mut self, idx: usize) -> &mut Primitive {
        let next_index = (idx + 1) % self.prims.len();
        &mut self.prims[next_index]
    }

    fn modify_vertex(&mut self, current: usize, dpos: Vec2, snap: f64) -> Option<f64> {
        // To calculate the new radius from dpos, we need to project dpos on
        // the bisector of the angle formed by the two lines
        let start_prev = self.get_prev_prim(current).get_start_pos();
        let end_prev = self.get_prev_prim(current).get_end_pos();
        let end = self.get_prim(current).get_end_pos();

        let (dpos_proj, sign) = project_onto_bisector(start_prev, end_prev, end, dpos);
        let radius_saved = self.get_prim(current).get_start_modifier_offset_saved();
        let radius = radius_saved + snap_val(dpos_proj.hypot() * sign, snap);
        if radius > 0. {
            self.get_prim_mut(current).set_start_modifier_offset(radius);
            Some(radius)
        } else {
            None
        }
    }
    fn move_vertex(&mut self, current: usize, dpos: Vec2, snap: f64, pointer_magnetized: bool) {
        // Vertex move: depends on the start property of the current primitive
        match self.get_prim(current).get_start_property() {
            VertexProperty::Nope => {
                let start_saved = self.get_prim(current).get_start_saved_pos();

                let new_pos = if !pointer_magnetized {
                    let prev_start = self.get_prev_prim(current).get_start_pos();
                    let end = self.get_prim(current).get_end_pos();
                    // We want the lengths of the two adjacent edges to be rounded
                    // at a multiple of snap
                    move_b_with_snapping(prev_start, start_saved, end, dpos, snap)
                } else {
                    start_saved + dpos
                };

                self.get_prim_mut(current).set_start_pos(new_pos);
                self.get_prev_prim_mut(current).set_end_pos(new_pos);

                // Update adjacent primitives controls
                self.get_prim_mut(current)
                    .update_primitives_vars(VertexChange::StartChanged);
                self.get_prev_prim_mut(current)
                    .update_primitives_vars(VertexChange::StartChanged);
            }
            VertexProperty::RectangleLike => {
                let s_saved = self.get_prim(current).get_start_saved_pos();
                let ps_saved = self.get_prev_prim(current).get_start_saved_pos();
                let ns_saved = self.get_next_prim(current).get_start_saved_pos();

                let pv = s_saved - ps_saved;
                let v = s_saved - ns_saved;
                if pv.hypot() < EPSILON || v.hypot() < EPSILON {
                    return;
                }
                let pv_norm = pv.normalize();
                let v_norm = v.normalize();
                let (dpos_proj_pv, dpos_proj_v) = get_coordinates_in_base(pv_norm, v_norm, dpos);

                let (new_ns, new_ps, new_s) = if !pointer_magnetized {
                    let new_pv = pv_norm * snap_val(pv.hypot() + dpos_proj_pv, snap);
                    let new_v = v_norm * snap_val(v.hypot() + dpos_proj_v, snap);

                    let new_ns = ns_saved + new_pv - pv;
                    let new_ps = ps_saved + new_v - v;

                    let new_s = new_ns + new_v;

                    (new_ns, new_ps, new_s)
                } else {
                    (
                        ns_saved + pv_norm * dpos_proj_pv,
                        ps_saved + v_norm * dpos_proj_v,
                        s_saved + pv_norm * dpos_proj_pv + v_norm * dpos_proj_v,
                    )
                };
                if (new_s - new_ps).hypot() > EPSILON && (new_s - new_ns).hypot() > EPSILON {
                    // Update start
                    self.get_prim_mut(current).set_start_pos(new_s);
                    self.get_prev_prim_mut(current).set_end_pos(new_s);

                    // Update next start
                    self.get_next_prim_mut(current).set_start_pos(new_ns);
                    self.get_prim_mut(current).set_end_pos(new_ns);

                    // Update prev start
                    self.get_prev_prim_mut(current).set_start_pos(new_ps);
                    self.get_prev_prev_prim_mut(current).set_end_pos(new_ps);

                    // Update adjacent primitives controls
                    self.get_prim_mut(current)
                        .update_primitives_vars(VertexChange::StartChanged);
                    self.get_prev_prim_mut(current)
                        .update_primitives_vars(VertexChange::StartChanged);
                }
            }
        }
    }

    fn hs_modifier_from_pos(&mut self, pointer: &mut Pointer, _keys_states: KeysStates, hs: HS) {
        use GetPrimitiveState::*;
        use SetPrimitiveState::*;
        use SetPrimitiveStateFromPos::*;
        // First, reset all modifiers
        self.prims.iter_mut().for_each(|prim| {
            prim.set_state(SetHS(hs, false));
            prim.set_state(SetStartHS(hs, false));
            prim.set_state(SetAllOtherModifiersHS(hs, false));
        });
        // Centroid pointer snap
        let centroid = self.get_vertices_centroid();
        if (pointer.pos() - centroid).hypot() < Self::GRAB_RADIUS {
            pointer.set_pos(centroid);
            if let HS::Select = hs {
                pointer.save_pos();
            }
            return;
        }
        //
        for prim in self.prims.iter_mut() {
            prim.set_state_from_pos(pointer, SetStartHSFromPos(hs));
            if prim.get_state(IsStartSelected).is_some() {
                return;
            }
            prim.set_state_from_pos(pointer, SetHSFromPos(hs));
            if prim.get_state(IsSelected).is_some() {
                return;
            }
            prim.set_state_from_pos(pointer, SetOthersModifiersHSFromPos(hs));
            if prim.get_state(IsOtherModifiersSelected).is_some() {
                return;
            }
        }

        // Fillets snap, get fillets centers and test
        use VertexModifier::*;
        let len = self.prims.len();
        for i in 0..len {
            let prim_prev = self.get_prev_prim(i);
            let prim: &Primitive = self.get_prim(i);

            let start_mod = prim.get_start_modifier();
            let start_modifier_offset = prim.get_start_modifier_offset();

            let start = prim.get_start_pos();
            let start_prev = prim_prev.get_start_pos();
            let end = prim.get_end_pos();
            let end_prev = prim_prev.get_end_pos();

            match prim.get_prim_curve() {
                PrimitiveCurve::CurveLine => {
                    let start_real = point_from_start(start, end, start_modifier_offset);
                    let prev_end_real = point_from_end(start_prev, end_prev, start_modifier_offset);

                    match start_mod {
                        Fillet(mut concavity) => {
                            let angle =
                                (PI - angle_from(start - prev_end_real, start_real - start)) * 0.5;
                            let mut radius = -start_modifier_offset * angle.tan();

                            if radius > 0. {
                                concavity = !concavity;
                                radius = -radius;
                            }
                            let center = create_arc_from_radius_and_concavity(
                                prev_end_real,
                                start_real,
                                radius,
                                concavity,
                            )
                            .center;
                            if (pointer.pos() - center.to_vec2()).hypot() < Self::GRAB_RADIUS {
                                match hs {
                                    HS::Select => {
                                        pointer.set_pos(center.to_vec2());
                                        pointer.save_pos();
                                    }
                                    HS::Highlight => {
                                        pointer.set_pos(center.to_vec2());
                                    }
                                }
                                return;
                            }
                        }
                        _ => (),
                    }
                }
                _ => (),
            };
        }
    }
    fn hs_all_modifiers(&mut self, value: bool, hs: HS) {
        use SetPrimitiveState::*;
        self.prims.iter_mut().for_each(|prim| {
            prim.set_state(SetHS(hs, value));
            prim.set_state(SetStartHS(hs, value));
            prim.set_state(SetAllOtherModifiersHS(hs, value));
        });
    }

    fn get_paths_patterns(&self) -> Vec<(BezPath, Pattern)> {
        use VertexModifier::*;
        let mut paths_patterns = vec![];
        let len = self.prims.len();
        for i in 0..len {
            let prim_prev = self.get_prev_prim(i);
            let prim: &Primitive = self.get_prim(i);
            let prim_next = self.get_next_prim(i);

            let start_mod = prim.get_start_modifier();
            let end_mod = prim_next.get_start_modifier();
            let start_modifier_offset = prim.get_start_modifier_offset();
            let end_modifier_offset = prim_next.get_start_modifier_offset();

            let start = prim.get_start_pos();
            let start_prev = prim_prev.get_start_pos();
            let end = prim.get_end_pos();
            let end_prev = prim_prev.get_end_pos();

            let prim_start_pattern = prim.get_pattern(
                prim.is_start_selected() || self.selected,
                prim.is_start_highlighted() || self.highlighted,
            );

            match prim.get_prim_curve() {
                PrimitiveCurve::CurveLine => {
                    let selected = prim.get_line().is_selected() || self.selected;
                    let highlighted = prim.get_line().is_highlighted() || self.highlighted;

                    let start_real = point_from_start(start, end, start_modifier_offset);
                    let end_real = point_from_end(start, end, end_modifier_offset);
                    let prev_end_real = point_from_end(start_prev, end_prev, start_modifier_offset);

                    match start_mod {
                        Nope(_) => {
                            let ee = if let Nope(_) = end_mod { end } else { end_real };
                            paths_patterns.push((
                                self.line_to(start, ee),
                                prim.get_pattern(selected, highlighted),
                            ));
                        }
                        Chamfer(_) => {
                            paths_patterns.push((
                                self.line_to(prev_end_real, start_real),
                                prim_start_pattern,
                            ));
                            let ee = if let Nope(_) = end_mod { end } else { end_real };
                            paths_patterns.push((
                                self.line_to(start_real, ee),
                                prim.get_pattern(selected, highlighted),
                            ));
                        }
                        Fillet(mut concavity) => {
                            let angle =
                                (PI - angle_from(start - prev_end_real, start_real - start)) * 0.5;
                            let mut radius = -start_modifier_offset * angle.tan();

                            if radius > 0. {
                                concavity = !concavity;
                                radius = -radius;
                            }
                            let f = create_arc_from_radius_and_concavity(
                                prev_end_real,
                                start_real,
                                radius,
                                concavity,
                            );
                            paths_patterns.push((f.into_path(Self::TOLERANCE), prim_start_pattern));
                            let ee = if let Nope(_) = end_mod { end } else { end_real };
                            paths_patterns.push((
                                self.line_to(start_real, ee),
                                prim.get_pattern(selected, highlighted),
                            ));
                        }
                    }
                }
                PrimitiveCurve::CurveArc => {
                    let selected = prim.get_arc().is_selected() || self.selected;
                    let highlighted = prim.get_arc().is_highlighted() || self.highlighted;
                    let radius = prim.get_arc().get_radius();
                    let concavity = prim.get_arc().get_concavity();
                    let f = create_arc_from_radius_and_concavity(start, end, radius, concavity);
                    paths_patterns.push((
                        f.into_path(Self::TOLERANCE),
                        prim.get_pattern(selected, highlighted),
                    ));
                }
            };
        }
        paths_patterns
    }
}
impl Display for ShapeCustom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Custom")
    }
}
impl Shape for ShapeCustom {
    type PathElementsIter<'iter> = ShapeCustomIter;

    fn path_elements(&self, _tolerance: f64) -> ShapeCustomIter {
        let mut iter = vec![];
        let paths = self.get_paths_patterns();
        for (bez_path, _) in paths.iter() {
            for el in bez_path.elements() {
                iter.push(*el);
            }
        }
        ShapeCustomIter { idx: 0, iter }
    }
    #[inline]
    fn area(&self) -> f64 {
        0.
    }
    #[inline]
    fn perimeter(&self, _accuracy: f64) -> f64 {
        0.
    }
    #[inline]
    fn winding(&self, pt: Point) -> i32 {
        compute_winding_number(&self.segs, pt.to_vec2())
    }
    #[inline]
    fn bounding_box(&self) -> Rect {
        self.get_bounding_box()
    }
    #[inline]
    fn as_rect(&self) -> Option<Rect> {
        None
    }
    #[inline]
    fn contains(&self, pt: Point) -> bool {
        self.winding(pt) != 0
    }
}
impl ObjectsFuncs for ShapeCustom {
    const TOLERANCE: f64 = 0.01;
    const GRAB_RADIUS: f64 = 5.;
    type Kindvars = MiiShapeVars;

    fn save_vars(&mut self) {
        self.prims.iter_mut().for_each(|prim| prim.save_vars());
    }
    fn restore_vars(&mut self) {
        self.prims.iter_mut().for_each(|prim| prim.restore_saved());
        self.update_polygon();
    }
    fn get_vars(&self) -> MiiShapeVars {
        let mut vars = vec![];
        self.prims.iter().for_each(|prim| {
            vars.push(prim.get_vars());
        });
        MiiShapeVars::MiiPolygon(vars)
    }
    fn set_vars(&mut self, vars: &MiiShapeVars) {
        if let MiiShapeVars::MiiPolygon(prim_vars) = vars {
            for (prim, prim_vars) in self.prims.iter_mut().zip(prim_vars.iter()) {
                prim.set_vars(prim_vars);
            }
            self.update_polygon();
        }
    }

    fn good_size(&self) -> bool {
        match self.primitivess_start_property {
            VertexProperty::Nope => {
                if self.prims.len() < 3 {
                    // Minimum segments was not reached
                    log!("Too few segments");
                    false
                } else {
                    true
                }
            }
            VertexProperty::RectangleLike => {
                let len = self.prims.len();
                if len < 4 {
                    return false;
                }
                let start = self.prims[0].get_start_pos();
                let end = self.prims[2].get_start_pos();
                let width = (end.x - start.x).abs();
                let height = (end.y - start.y).abs();
                width >= Self::MIN_RECT_SIZE && height >= Self::MIN_RECT_SIZE
            }
        }
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
            IsAnyControlSelected => {
                if self
                    .prims
                    .iter()
                    .any(|prim| prim.get_state(GetPrimitiveState::IsSelected).is_some())
                {
                    return Some(self.get_position());
                } else {
                    if self
                        .prims
                        .iter()
                        .any(|prim| prim.get_state(GetPrimitiveState::IsStartSelected).is_some())
                    {
                        return Some(self.get_position());
                    } else {
                        if self.prims.iter().any(|prim| {
                            prim.get_state(GetPrimitiveState::IsOtherModifiersSelected)
                                .is_some()
                        }) {
                            return Some(self.get_position());
                        } else {
                            return None;
                        }
                    }
                }
            }
            IsAnyControlHighligh => {
                if self
                    .prims
                    .iter()
                    .any(|prim| prim.get_state(GetPrimitiveState::IsHighligh).is_some())
                {
                    return Some(self.get_position());
                } else {
                    if self
                        .prims
                        .iter()
                        .any(|prim| prim.get_state(GetPrimitiveState::IsStartHighligh).is_some())
                    {
                        return Some(self.get_position());
                    } else {
                        if self.prims.iter().any(|prim| {
                            prim.get_state(GetPrimitiveState::IsOtherModifiersHighligh)
                                .is_some()
                        }) {
                            return Some(self.get_position());
                        } else {
                            return None;
                        }
                    }
                }
            }
        }
    }
    fn set_state(&mut self, set: SetEntityState) {
        match set {
            SetEntityState::SetHighli(value) => self.highlighted = value,
            SetEntityState::SetSelect(value) => self.selected = value,
            SetEntityState::SelectAllControls(value) => self.hs_all_modifiers(value, HS::Select),
            SetEntityState::HighliAllControls(value) => self.hs_all_modifiers(value, HS::Highlight),
        }
    }
    fn set_state_from_pos(
        &mut self,
        pointer: &mut Pointer,
        keys_states: KeysStates,
        set: SetEntityStateFromPos,
    ) {
        match set {
            SetEntityStateFromPos::HighliFromPos => {
                self.highlighted = self.contains(pointer.pos().to_point());
            }
            SetEntityStateFromPos::SelectFromPos => {
                self.selected = self.contains(pointer.pos().to_point())
            }
            SetEntityStateFromPos::SelectControlFromPos => {
                self.hs_modifier_from_pos(pointer, keys_states, HS::Select);
            }
            SetEntityStateFromPos::HighliControlFromPos => {
                self.hs_modifier_from_pos(pointer, keys_states, HS::Highlight);
            }
        }
    }

    fn toggle_prop(&mut self) {
        ()
    }

    fn move_position(&mut self, pointer: &mut Pointer, _keys_states: KeysStates) -> bool {
        let mut moved = false;
        self.prims.iter_mut().for_each(|prim| {
            moved |= prim.move_position(pointer);
        });
        self.update_polygon();
        moved
    }
    fn move_modifier(&mut self, pointer: &Pointer, keys_states: KeysStates) -> bool {
        // Check if we are in creation mode
        if let Some(current_pos) = &mut self.current_creation_pos {
            match self.primitivess_start_property {
                VertexProperty::Nope => {
                    // Yes, update the last line
                    if let Some(last_line) = self.prims.last_mut() {
                        if let PrimitiveCurve::CurveLine = last_line.get_prim_curve() {
                            let start_pos = last_line.get_start_pos();
                            if !pointer.is_magnetized() {
                                current_pos.pos = current_pos.saved_pos + pointer.dpos();
                                current_pos.pos =
                                    snap_pt(current_pos.pos - start_pos, pointer.get_snap().val())
                                        + start_pos;
                            } else {
                                current_pos.pos = current_pos.saved_pos + pointer.dpos();
                            }
                            last_line.set_end_pos(current_pos.pos);
                        }
                    }
                    self.update_polygon();
                    true
                }
                VertexProperty::RectangleLike => {
                    let snap = pointer.get_snap().val();
                    let dpos = pointer.dpos();
                    let magnetized = pointer.is_magnetized();
                    // Vertex move
                    self.move_vertex(2, dpos, snap, magnetized);
                    self.update_polygon();
                    return true;
                }
            }
        } else {
            // Move the first polygon vertex found in case of multiples (normally not the case)
            // Also, since each primitive has start/end vertices, we need to move the end vertex
            // of the previous primitive
            let snap = pointer.get_snap().val();
            let dpos = pointer.dpos();
            let magnetized = pointer.is_magnetized();
            let mut vertex_modified = None;
            for current in 0..self.prims.len() {
                if self.get_prim(current).is_start_selected() {
                    if keys_states.crtl_cmd_pressed {
                        // Vertex modification (size of chamfer or fillet)
                        let start_mod = self.get_prim(current).get_start_modifier();
                        match start_mod {
                            VertexModifier::Nope(_) => {
                                continue;
                            }
                            VertexModifier::Chamfer(_) | VertexModifier::Fillet(_) => {
                                if let Some(radius) = self.modify_vertex(current, dpos, snap) {
                                    vertex_modified = Some((radius, start_mod));
                                    self.update_polygon();
                                    break;
                                }
                            }
                        }
                    } else {
                        // Vertex move
                        self.move_vertex(current, dpos, snap, magnetized);
                        self.update_polygon();
                        return true;
                    }
                }
            }
            if let Some((radius, start_mod)) = vertex_modified {
                if keys_states.crtl_cmd_pressed && keys_states.shift_pressed {
                    for prim in self.prims.iter_mut() {
                        prim.set_start_modifier(start_mod);
                        prim.set_start_modifier_offset(radius);
                    }
                    self.update_polygon();
                    return true;
                }
            }

            // Move prim modifiers if selected
            let mut moved = false;
            for prim in self.prims.iter_mut() {
                if prim.move_control_selected(pointer, keys_states) {
                    self.update_polygon();
                    moved = true;
                    break;
                }
            }
            moved
        }
    }

    fn get_position(&self) -> Vec2 {
        self.get_vertices_centroid()
    }

    fn get_mod_paths_and_patterns(
        &self,
        das: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        let mut paths_patterns: Vec<(BezPath, Pattern)> = vec![];
        for prim in self.prims.iter() {
            paths_patterns.extend(prim.get_mod_paths_and_patterns(das));
        }
        paths_patterns.push((
            center_path(self.get_position(), 1., ShapeCustom::GRAB_RADIUS),
            self.get_pattern_status(self.selected, self.highlighted),
        ));
        // Filets centers
        use VertexModifier::*;
        let len = self.prims.len();
        for i in 0..len {
            let prim_prev = self.get_prev_prim(i);
            let prim: &Primitive = self.get_prim(i);
            let prim_next = self.get_next_prim(i);

            let start_mod = prim.get_start_modifier();
            let end_mod = prim_next.get_start_modifier();
            let start_modifier_offset = prim.get_start_modifier_offset();
            let end_modifier_offset = prim_next.get_start_modifier_offset();

            let start = prim.get_start_pos();
            let start_prev = prim_prev.get_start_pos();
            let end = prim.get_end_pos();
            let end_prev = prim_prev.get_end_pos();

            let prim_start_pattern = prim.get_pattern(
                prim.is_start_selected() || self.selected,
                prim.is_start_highlighted() || self.highlighted,
            );

            match prim.get_prim_curve() {
                PrimitiveCurve::CurveLine => {
                    let selected = prim.get_line().is_selected() || self.selected;
                    let highlighted = prim.get_line().is_highlighted() || self.highlighted;

                    let start_real = point_from_start(start, end, start_modifier_offset);
                    let end_real = point_from_end(start, end, end_modifier_offset);
                    let prev_end_real = point_from_end(start_prev, end_prev, start_modifier_offset);

                    match start_mod {
                        Nope(_) => (),
                        Chamfer(_) => (),
                        Fillet(mut concavity) => {
                            let angle =
                                (PI - angle_from(start - prev_end_real, start_real - start)) * 0.5;
                            let mut radius = -start_modifier_offset * angle.tan();

                            if radius > 0. {
                                concavity = !concavity;
                                radius = -radius;
                            }
                            let center = create_arc_from_radius_and_concavity(
                                prev_end_real,
                                start_real,
                                radius,
                                concavity,
                            )
                            .center
                            .to_vec2();
                            paths_patterns.push((
                                center_path(center, 1., Self::GRAB_RADIUS),
                                prim_start_pattern,
                            ));
                            let ee = if let Nope(_) = end_mod { end } else { end_real };
                            paths_patterns.push((
                                self.line_to(start_real, ee),
                                prim.get_pattern(selected, highlighted),
                            ));
                        }
                    }
                }
                _ => (),
            };
        }
        paths_patterns
    }
    fn get_dimensions_paths_and_patterns(
        &self,
        size: &Size,
        _cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        use GetPrimitiveState::*;
        let mut res = vec![];
        for prim in self.prims.iter() {
            if prim.get_state(IsHighligh).is_some() || prim.get_state(IsSelected).is_some() {
                let dim = prim.get_dimensions_paths_and_patterns(size);
                res.extend(dim);
            }
        }
        res
    }
    fn get_paths_and_patterns(&self, _: &Size, _: (Rect, f64, Vec2)) -> Vec<(BezPath, Pattern)> {
        self.get_paths_patterns()
    }
}

pub struct ShapeCustomIter {
    idx: usize,
    iter: Vec<PathEl>,
}
impl Iterator for ShapeCustomIter {
    type Item = PathEl;

    fn next(&mut self) -> Option<Self::Item> {
        self.idx += 1;
        self.iter.get(self.idx - 1).cloned()
    }
}
