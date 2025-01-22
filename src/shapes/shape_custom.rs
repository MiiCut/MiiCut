// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
use super::shapes::{BSKind, BSKindvars};
use crate::{
    canvas::{CanvasText, Pattern},
    math::*,
    positions::Position,
    prefab::center_path,
    primitives::primitives::{
        GetPrimitiveState, Privitive, PrivitiveKind, SetPrimitiveState, SetPrimitiveStateFromPos,
    },
    traits::*,
    Pointer,
};
use geo::{LineString, Polygon};
use kurbo::{BezPath, PathEl, Point, Rect, Shape, Size, Vec2};
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct ShapeCustom {
    d1s: Vec<Privitive>,
    current_creation_pos: Option<Position>,

    highlighted: bool,
    selected: bool,

    segs: BezPath,
    polygon: Polygon<f64>,
}
impl ShapeCustom {
    const _MIN_SIZE: f64 = 10.;

    pub fn new(pos1: Vec2, pos2: Vec2) -> BSKind {
        BSKind::Custom(ShapeCustom {
            d1s: vec![Privitive::new(PrivitiveKind::D1KLine, pos1, pos2)],
            current_creation_pos: Some(Position::new(pos2, true)),
            highlighted: false,
            selected: false,
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }
    pub fn add_point(&mut self, pointer: &mut Pointer) {
        let pos = pointer.pos();
        // Get the last line drawn
        if let Some(last_line) = self.d1s.last_mut() {
            if let PrivitiveKind::D1KLine = last_line.get_d1_kind() {
                if let Some(current_pos) = &mut self.current_creation_pos {
                    current_pos.pos = pos;
                    current_pos.saved_pos = pos;
                    last_line.set_end_position(pos);
                    self.d1s
                        .push(Privitive::new(PrivitiveKind::D1KLine, pos, pos));
                    self.update_polygon();
                }
            }
        }
    }
    pub fn end_creation(&mut self) -> bool {
        if self.good_size() {
            self.current_creation_pos = None;
            let first_pos = self.d1s.first().unwrap().get_start_position();
            if let Some(last_d1) = self.d1s.last_mut() {
                if let PrivitiveKind::D1KLine = last_d1.get_d1_kind() {
                    last_d1.set_end_position(first_pos);
                }
            }
            self.update_polygon();
            true
        } else {
            // Minimum segments was not reached
            log!("Too few segments");
            false
        }
    }
    pub fn get_polygon(&self) -> Polygon<f64> {
        self.polygon.clone()
    }
    pub fn update_polygon(&mut self) {
        self.segs = calc_segs(self.get_paths(&Size::ZERO));
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
        if self.d1s.is_empty() {
            return Rect::new(0., 0., 0., 0.);
        }

        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for d1kind in self.d1s.iter() {
            let start = d1kind.get_start_position();
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
    pub fn get_d1s_mut(&mut self) -> &mut Vec<Privitive> {
        &mut self.d1s
    }
    fn get_vertices_centroid(&self) -> Vec2 {
        let mut centroid = Vec2::ZERO;
        self.d1s.iter().for_each(|d1kind| {
            centroid += d1kind.get_start_position();
        });
        centroid / self.d1s.len() as f64
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
        for d1kind in self.d1s.iter() {
            let path = d1kind.to_path();
            for el in path {
                iter.push(el);
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
    type Kindvars = BSKindvars;

    fn save_vars(&mut self) {
        self.d1s.iter_mut().for_each(|d1kind| d1kind.save_vars());
    }
    fn restore_saved(&mut self) {
        self.d1s
            .iter_mut()
            .for_each(|d1kind| d1kind.restore_saved());
        self.update_polygon();
    }
    fn get_vars(&self) -> BSKindvars {
        let mut vars = vec![];
        self.d1s.iter().for_each(|d1kind| {
            vars.push(d1kind.get_vars());
        });
        BSKindvars::Custom(vars)
    }
    fn set_vars(&mut self, vars: &BSKindvars) {
        if let BSKindvars::Custom(d1vars) = vars {
            for (d1kind, d1kind_vars) in self.d1s.iter_mut().zip(d1vars.iter()) {
                d1kind.set_vars(d1kind_vars);
            }
            self.update_polygon();
        }
    }

    fn good_size(&self) -> bool {
        self.d1s.len() >= 3
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
                if self
                    .d1s
                    .iter()
                    .any(|d1kind| d1kind.get_state(GetPrimitiveState::IsSelected).is_some())
                {
                    return Some(self.get_position());
                } else {
                    if self.d1s.iter().any(|d1kind| {
                        d1kind
                            .get_state(GetPrimitiveState::IsStartSelected)
                            .is_some()
                    }) {
                        return Some(self.get_position());
                    } else {
                        if self.d1s.iter().any(|d1kind| {
                            d1kind
                                .get_state(GetPrimitiveState::IsOtherModifiersSelected)
                                .is_some()
                        }) {
                            return Some(self.get_position());
                        } else {
                            return None;
                        }
                    }
                }
            }
            IsAnyModifierHighligh => {
                if self
                    .d1s
                    .iter()
                    .any(|d1kind| d1kind.get_state(GetPrimitiveState::IsHighligh).is_some())
                {
                    return Some(self.get_position());
                } else {
                    if self.d1s.iter().any(|d1kind| {
                        d1kind
                            .get_state(GetPrimitiveState::IsStartHighligh)
                            .is_some()
                    }) {
                        return Some(self.get_position());
                    } else {
                        if self.d1s.iter().any(|d1kind| {
                            d1kind
                                .get_state(GetPrimitiveState::IsOtherModifiersHighligh)
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
        use SetPrimitiveState::*;
        match set {
            SetEntityState::SetHighli(value) => self.highlighted = value,
            SetEntityState::SetSelect(value) => self.selected = value,
            SetEntityState::SelectAllModifiers(value) => {
                self.d1s.iter_mut().for_each(|d1kind| {
                    d1kind.set_state(SetSelect(value));
                    d1kind.set_state(SetStartSelected(value));
                    d1kind.set_state(SelectAllOtherModifiers(value));
                });
            }
            SetEntityState::HighliAllModifiers(value) => {
                self.d1s.iter_mut().for_each(|d1kind| {
                    d1kind.set_state(SetHighli(value));
                    d1kind.set_state(SetStartHighligh(value));
                    d1kind.set_state(HighliAllOtherModifiers(value));
                });
            }
        }
    }
    fn set_state_from_pos(&mut self, pointer: &mut Pointer, set: SetEntityStateFromPos) {
        use GetPrimitiveState::*;
        use SetPrimitiveState::*;
        use SetPrimitiveStateFromPos::*;
        match set {
            SetEntityStateFromPos::HighliFromPos => {
                self.highlighted = self.contains(pointer.pos().to_point());
            }
            SetEntityStateFromPos::SelectFromPos => {
                self.selected = self.contains(pointer.pos().to_point())
            }
            SetEntityStateFromPos::SelectModifierFromPos => {
                self.d1s.iter_mut().for_each(|d1| {
                    d1.set_state(SetSelect(false));
                    d1.set_state(SetStartSelected(false));
                    d1.set_state(SelectAllOtherModifiers(false));
                });

                for d1 in self.d1s.iter_mut() {
                    d1.set_state_from_pos(pointer, SelectStartFromPos);
                    if d1.get_state(IsStartSelected).is_some() {
                        break;
                    }
                    d1.set_state_from_pos(pointer, SelectFromPos);
                    if d1.get_state(IsSelected).is_some() {
                        break;
                    }
                    d1.set_state_from_pos(pointer, SelectOtherModifierFromPos);
                    if d1.get_state(IsOtherModifiersSelected).is_some() {
                        break;
                    }
                }
            }
            SetEntityStateFromPos::HighliModifierFromPos => {
                self.d1s.iter_mut().for_each(|d1| {
                    d1.set_state(SetHighli(false));
                    d1.set_state(SetStartHighligh(false));
                    d1.set_state(HighliAllOtherModifiers(false));
                });

                for d1 in self.d1s.iter_mut() {
                    d1.set_state_from_pos(pointer, HighliStartFromPos);
                    if d1.get_state(IsStartHighligh).is_some() {
                        break;
                    }
                    d1.set_state_from_pos(pointer, HighliFromPos);
                    if d1.get_state(IsHighligh).is_some() {
                        break;
                    }
                    d1.set_state_from_pos(pointer, HighliOtherModifierFromPos);
                    if d1.get_state(IsOtherModifiersHighligh).is_some() {
                        break;
                    }
                }
            }
        }
    }

    fn toggle_prop(&mut self) {
        ()
    }

    fn move_position(&mut self, pointer: &mut Pointer, _shift_pressed: bool) -> bool {
        let mut moved = false;
        self.d1s.iter_mut().for_each(|d1kind| {
            moved |= d1kind.move_position(pointer);
        });
        self.update_polygon();
        moved
    }
    fn move_modifier(&mut self, pointer: &mut Pointer, shift_pressed: bool) -> bool {
        //  let dpos = snap_pt(pos - pos_init, snap);
        // Check if we are in creation mode
        if let Some(current_pos) = &mut self.current_creation_pos {
            current_pos.pos = current_pos.saved_pos + pointer.dpos();
            // Update the last line
            if let Some(last_line) = self.d1s.last_mut() {
                if let PrivitiveKind::D1KLine = last_line.get_d1_kind() {
                    last_line.set_end_position(current_pos.pos);
                }
            }
            pointer.set_pos(current_pos.pos);
            self.update_polygon();
            true
        } else {
            // Move the first polygon vertex found in case of multiples (normally not the case)
            // Also, since each primitive has start/end vertices, we need to move the end vertex
            // of the previous primitive
            let len = self.d1s.len();
            let dpos = pointer.dpos();
            let snap = pointer.get_snap().val();
            for i in 0..self.d1s.len() {
                if self.d1s[i].is_start_selected() {
                    let pos_saved = self.d1s[i].get_start_saved_position();
                    self.d1s[i].set_start_position(snap_pt(pos_saved + dpos, snap));
                    let prev_index = if i == 0 { len - 1 } else { (i - 1) % len };
                    self.d1s[prev_index].set_end_position(snap_pt(pos_saved + dpos, snap));
                    self.update_polygon();
                    pointer.set_pos(self.d1s[i].get_start_position());
                    return true;
                }
            }

            // Move d1 modifiers if selected
            let mut moved = false;
            for d1 in self.d1s.iter_mut() {
                if d1.move_control_selected(pointer, shift_pressed) {
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
    fn get_dimensions_paths_and_patterns(
        &self,
        size: &Size,
        _cinfo: (Rect, f64, Vec2),
    ) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        use GetPrimitiveState::*;
        let mut paths_patterns = vec![];
        let mut texts = vec![];
        for d1 in self.d1s.iter() {
            if d1.get_state(IsHighligh).is_some() || d1.get_state(IsSelected).is_some() {
                let (path_pattern, text) = d1.get_dimensions_paths_and_patterns(size);
                paths_patterns.extend(path_pattern);
                texts.extend(text);
            }
        }
        (paths_patterns, texts)
    }

    fn get_paths(&self, _: &Size) -> Vec<BezPath> {
        let mut paths: Vec<BezPath> = vec![];
        for d1kind in self.d1s.iter() {
            paths.push(d1kind.to_path());
        }
        paths
    }
    fn get_paths_and_patterns(
        &self,
        das: &Size,
        _cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        let mut paths_patterns: Vec<(BezPath, Pattern)> = vec![];
        for d1kind in self.d1s.iter() {
            let path_pattern = d1kind.get_paths_and_patterns(das);
            if self.selected {
                paths_patterns.push((path_pattern.0, Pattern::BasicSelected));
            } else if self.highlighted {
                paths_patterns.push((path_pattern.0, Pattern::BasicHighlighted));
            } else {
                paths_patterns.push(path_pattern);
            }
        }
        paths_patterns
    }
    fn get_mod_paths_and_patterns(
        &self,
        das: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        let mut paths_patterns: Vec<(BezPath, Pattern)> = vec![];
        for d1kind in self.d1s.iter() {
            paths_patterns.extend(d1kind.get_mod_paths_and_patterns(das));
        }
        paths_patterns.push((
            center_path(self.get_position(), 1., ShapeCustom::GRAB_RADIUS),
            self.get_pattern_status(self.selected, self.highlighted),
        ));
        paths_patterns
    }
}

pub struct ShapeCustomIter {
    idx: usize,
    iter: Vec<PathEl>,
}
impl Iterator for ShapeCustomIter {
    type Item = PathEl;

    fn next(&mut self) -> Option<Self::Item> {
        log!("ShapeCustomIter next");
        self.idx += 1;
        self.iter.get(self.idx - 1).cloned()
    }
}
