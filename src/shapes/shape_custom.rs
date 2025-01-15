// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
use super::shapes::{BSKind, BSKindvars};
use crate::{
    canvas::{CanvasText, Pattern},
    d1::d1::{D1Kind, D1},
    math::*,
    positions::Position,
    prefab::{center_path, modifiers_path},
    traits::*,
};
use geo::{LineString, Polygon};
use kurbo::{BezPath, PathEl, Point, Rect, Shape, Size, Vec2};
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct ShapeCustom {
    d1s: Vec<D1>,
    current_creation_pos: Option<Position>,

    highlighted: bool,
    selected: bool,

    segs: BezPath,
    polygon: Polygon<f64>,
}
impl ShapeCustom {
    const MIN_SIZE: f64 = 10.;

    pub fn new(pos1: Vec2, pos2: Vec2) -> BSKind {
        BSKind::Custom(ShapeCustom {
            d1s: vec![D1::new(D1Kind::D1KLine, pos1, pos2)],
            current_creation_pos: Some(Position::new(pos2, true)),
            highlighted: false,
            selected: false,
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }
    pub fn add_point(&mut self, pos: Vec2) {
        // Get the last line drawn
        if let Some(last_line) = self.d1s.last_mut() {
            if let D1Kind::D1KLine = last_line.get_kind() {
                if let Some(current_pos) = &mut self.current_creation_pos {
                    current_pos.pos = pos;
                    current_pos.saved_pos = pos;
                    last_line.set_end(pos);
                    self.d1s.push(D1::new(D1Kind::D1KLine, pos, pos));
                    self.update_polygon();
                }
            }
        }
    }
    pub fn end_creation(&mut self) -> bool {
        if self.good_size() {
            self.current_creation_pos = None;
            let first_pos = self.d1s.first().unwrap().get_positions().0.pos;
            if let Some(last_d1) = self.d1s.last_mut() {
                if let D1Kind::D1KLine = last_d1.get_kind() {
                    last_d1.set_end(first_pos);
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
    fn update_polygon(&mut self) {
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
            let (start, _) = d1kind.get_positions();
            let start = start.pos;
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
            IsHighlighted => {
                if self.highlighted {
                    Some(self.get_position())
                } else {
                    None
                }
            }
            IsAnyModifierSelected => {
                let selected = self
                    .d1s
                    .iter()
                    .any(|d1kind| d1kind.get_state(IsAnyModifierSelected).is_some());
                if selected {
                    Some(self.get_position())
                } else {
                    None
                }
            }
            IsAnyModifierHighlighted => {
                let highlighted = self
                    .d1s
                    .iter()
                    .any(|d1kind| d1kind.get_state(IsAnyModifierHighlighted).is_some());
                if highlighted {
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
            SetHighlight(value) => self.highlighted = value,
            HighlightFromPos(pos, _snap, _grab) => self.highlighted = self.contains(pos.to_point()),

            SetSelect(value) => self.selected = value,
            SelectFromPos(pos, _snap, _grab) => self.selected = self.contains(pos.to_point()),

            SelectAllModifiers(value) => {
                self.d1s.iter_mut().for_each(|d1kind| {
                    d1kind.set_state(SelectAllModifiers(value));
                });
            }
            SelectModifierFromPos(pos, snap, grab) => {
                self.d1s.iter_mut().for_each(|d1kind| {
                    d1kind.set_state(SelectAllModifiers(false));
                    d1kind.set_state(SetSelect(false));
                });

                for d1kind in self.d1s.iter_mut() {
                    d1kind.set_state(SelectModifierFromPos(pos, snap, grab));
                }
                for d1kind in self.d1s.iter_mut() {
                    d1kind.set_state(SelectFromPos(pos, snap, grab));
                }
            }

            HighlightAllModifiers(value) => {
                self.d1s.iter_mut().for_each(|d1kind| {
                    d1kind.set_state(HighlightAllModifiers(value));
                });
            }
            HighlightModifierFromPos(pos, snap, grab) => {
                self.d1s.iter_mut().for_each(|d1kind| {
                    d1kind.set_state(HighlightModifierFromPos(pos, snap, grab));
                    d1kind.set_state(SetHighlight(false));
                });
            }
        }
    }

    fn toggle_prop(&mut self) {
        ()
    }

    fn move_position(&mut self, mut dpos: Vec2, snap: f64) -> Option<Vec2> {
        dpos = snap_pt(dpos, snap);
        self.d1s.iter_mut().for_each(|d1kind| {
            let saved_pos = d1kind.get_positions().0.saved_pos;
            d1kind.get_positions_mut().0.pos = saved_pos + dpos;
            let saved_pos = d1kind.get_positions().1.saved_pos;
            d1kind.get_positions_mut().1.pos = saved_pos + dpos;
        });
        self.update_polygon();
        None
    }
    fn move_modifier(
        &mut self,
        pos_init: Vec2,
        pos: Vec2,
        snap: f64,
        _shift_pressed: bool,
    ) -> Option<Vec2> {
        let dpos = snap_pt(pos - pos_init, snap);
        // Check if we are in creation mode
        if let Some(current_pos) = &mut self.current_creation_pos {
            current_pos.pos = current_pos.saved_pos + dpos;
            // Update the last line
            if let Some(last_line) = self.d1s.last_mut() {
                if let D1Kind::D1KLine = last_line.get_kind() {
                    last_line.set_end(current_pos.pos);
                }
            }
            let pos = current_pos.pos;
            self.update_polygon();
            Some(pos)
        } else {
            // Move the first hs found in case of multiple (normally not the case)
            let len = self.d1s.len();
            let mut pos_moved = None;
            for i in 0..self.d1s.len() {
                if self.d1s[i].get_positions_mut().0.selected {
                    let pos_saved = self.d1s[i].get_positions().0.saved_pos;
                    self.d1s[i].get_positions_mut().0.pos = snap_pt(pos_saved + dpos, snap);
                    let prev_index = if i == 0 { len - 1 } else { (i - 1) % len };
                    self.d1s[prev_index].get_positions_mut().1.pos =
                        snap_pt(pos_saved + dpos, snap);
                    self.update_polygon();
                    pos_moved = Some(self.d1s[i].get_positions_mut().0.pos);
                    break;
                }
            }
            pos_moved
        }
    }
    fn get_position(&self) -> Vec2 {
        // Geometrical center (from line segments)
        let mut geo_center = Vec2::ZERO;
        self.d1s.iter().for_each(|d1kind| {
            let (start, _) = d1kind.get_positions();
            geo_center += start.pos;
        });
        geo_center / self.d1s.len() as f64
    }
    fn get_modifiers_paths(&self, _: &Size) -> Vec<(BezPath, Pattern)> {
        let mut paths: Vec<(BezPath, Pattern)> = vec![];
        for d1kind in self.d1s.iter() {
            let pos = d1kind.get_positions_into().0;
            paths.push((
                modifiers_path(pos.pos, 1., ShapeCustom::GRAB_RADIUS),
                self.get_pattern_modifiers(pos.selected, pos.highlighted),
            ));
        }
        paths.push((
            center_path(self.get_position(), 1., ShapeCustom::GRAB_RADIUS),
            self.get_pattern_modifiers(self.selected, self.highlighted),
        ));
        paths
    }
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        let mut _paths = vec![];
        let mut _texts = vec![];
        // let (path, text) = Dimension::new(
        //     DimKind::Horizontal,
        //     self.tl.get_pos(),
        //     self.get_tr_modifier(),
        //     self.get_width(),
        // )
        // .get_path();
        // paths.push(path);
        // texts.push(text);

        // let (path, text) = Dimension::new(
        //     DimKind::Vertical,
        //     self.get_bl_modifier(),
        //     self.tl.get_pos(),
        //     self.get_height(),
        // )
        // .get_path();
        // paths.push(path);
        // texts.push(text);
        (_paths, _texts)
    }
    fn get_paths(&self, _: &Size) -> Vec<BezPath> {
        let mut paths: Vec<BezPath> = vec![];
        for d1kind in self.d1s.iter() {
            paths.push(d1kind.to_path());
        }
        paths
    }
    fn get_paths_and_patterns(&self, drawing_area_size: &Size) -> Vec<(BezPath, Pattern)> {
        let mut paths_patterns: Vec<(BezPath, Pattern)> = vec![];
        for d1kind in self.d1s.iter() {
            let path_pattern = d1kind.get_paths_and_patterns(drawing_area_size);
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
