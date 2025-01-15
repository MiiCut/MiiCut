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
    positions::Position,
    prefab::{center_path, modifiers_path},
    traits::*,
    Modifier,
};
use geo::{LineString, Polygon};
use kurbo::{BezPath, PathEl, Point, Rect, RectPathIter, Shape, Size, Vec2};
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq)]
pub struct ShapeRectangle {
    tl: Position,
    br: Position,

    tr: Modifier,
    bl: Modifier,
    top: Modifier,
    right: Modifier,
    bottom: Modifier,
    left: Modifier,

    highlighted: bool,
    selected: bool,

    segs: BezPath,
    polygon: Polygon<f64>,
}
impl ShapeRectangle {
    const MIN_SIZE: f64 = 10.;

    pub fn new(pos1: Vec2, pos2: Vec2) -> BSKind {
        let tl = Position::new(pos1, true);
        let mut br = Position::new(pos2, true);
        br.selected = true;

        BSKind::Rectangle(ShapeRectangle {
            tl,
            br,
            tr: Modifier::new(),
            bl: Modifier::new(),
            top: Modifier::new(),
            right: Modifier::new(),
            bottom: Modifier::new(),
            left: Modifier::new(),
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
    fn get_width(&self) -> f64 {
        (self.tl.pos.x - self.br.pos.x).abs()
    }
    fn get_height(&self) -> f64 {
        (self.tl.pos.y - self.br.pos.y).abs()
    }

    fn get_rectangle(&self) -> Rect {
        let tl_pos = self.tl.pos;
        let br_pos = self.br.pos;
        Rect::new(tl_pos.x, tl_pos.y, br_pos.x, br_pos.y)
    }

    fn get_tr_modifier(&self) -> Vec2 {
        Vec2::new(self.br.pos.x, self.tl.pos.y)
    }
    fn get_bl_modifier(&self) -> Vec2 {
        Vec2::new(self.tl.pos.x, self.br.pos.y)
    }

    fn get_top_modifier(&self) -> Vec2 {
        (self.tl.pos + self.get_tr_modifier()) / 2.
    }
    fn get_right_modifier(&self) -> Vec2 {
        (self.get_tr_modifier() + self.br.pos) / 2.
    }
    fn get_bottom_modifier(&self) -> Vec2 {
        (self.br.pos + self.get_bl_modifier()) / 2.
    }
    fn get_left_modifier(&self) -> Vec2 {
        (self.get_bl_modifier() + self.tl.pos) / 2.
    }

    fn get_tr_saved_modifier(&self) -> Vec2 {
        Vec2::new(self.br.saved_pos.x, self.tl.saved_pos.y)
    }
    fn get_bl_saved_modifier(&self) -> Vec2 {
        Vec2::new(self.tl.saved_pos.x, self.br.saved_pos.y)
    }

    fn get_top_saved_modifier(&self) -> Vec2 {
        (self.tl.saved_pos + self.get_tr_saved_modifier()) / 2.
    }
    fn get_right_saved_modifier(&self) -> Vec2 {
        (self.get_tr_saved_modifier() + self.br.saved_pos) / 2.
    }
    fn get_bottom_saved_modifier(&self) -> Vec2 {
        (self.br.saved_pos + self.get_bl_saved_modifier()) / 2.
    }
    fn get_left_saved_modifier(&self) -> Vec2 {
        (self.get_bl_saved_modifier() + self.tl.saved_pos) / 2.
    }

    fn highlight_all_modifiers(&mut self, value: bool) {
        self.tl.highlighted = value;
        self.tr.highlighted = value;
        self.br.highlighted = value;
        self.bl.highlighted = value;
        self.top.highlighted = value;
        self.right.highlighted = value;
        self.bottom.highlighted = value;
        self.left.highlighted = value;
    }
    fn select_all_modifiers(&mut self, value: bool) {
        self.tl.selected = value;
        self.tr.selected = value;
        self.br.selected = value;
        self.bl.selected = value;
        self.top.selected = value;
        self.right.selected = value;
        self.bottom.selected = value;
        self.left.selected = value;
    }

    fn highlight_modifiers_from_pos(&mut self, pos: Vec2, grab: f64) {
        self.tl.highlighted = (pos - self.tl.pos).hypot() < grab;
        self.tr.highlighted = (pos - self.get_tr_modifier()).hypot() < grab;
        self.br.highlighted = (pos - self.br.pos).hypot() < grab;
        self.bl.highlighted = (pos - self.get_bl_modifier()).hypot() < grab;
        self.top.highlighted = (pos - self.get_top_modifier()).hypot() < grab;
        self.right.highlighted = (pos - self.get_right_modifier()).hypot() < grab;
        self.bottom.highlighted = (pos - self.get_bottom_modifier()).hypot() < grab;
        self.left.highlighted = (pos - self.get_left_modifier()).hypot() < grab;
    }
    fn select_modifiers_from_pos(&mut self, pos: Vec2, grab: f64) {
        self.tl.selected = (pos - self.tl.pos).hypot() < grab;
        self.tr.selected = (pos - self.get_tr_modifier()).hypot() < grab;
        self.br.selected = (pos - self.br.pos).hypot() < grab;
        self.bl.selected = (pos - self.get_bl_modifier()).hypot() < grab;
        self.top.selected = (pos - self.get_top_modifier()).hypot() < grab;
        self.right.selected = (pos - self.get_right_modifier()).hypot() < grab;
        self.bottom.selected = (pos - self.get_bottom_modifier()).hypot() < grab;
        self.left.selected = (pos - self.get_left_modifier()).hypot() < grab;
    }
}
impl Display for ShapeRectangle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rectangle")
    }
}
impl Shape for ShapeRectangle {
    type PathElementsIter<'iter> = ShapeRectangleIter;

    fn path_elements(&self, tolerance: f64) -> ShapeRectangleIter {
        ShapeRectangleIter {
            rect_path_iter: self.get_rectangle().path_elements(tolerance),
        }
    }
    #[inline]
    fn area(&self) -> f64 {
        self.get_rectangle().area()
    }
    #[inline]
    fn perimeter(&self, accuracy: f64) -> f64 {
        self.get_rectangle().perimeter(accuracy)
    }
    #[inline]
    fn winding(&self, pt: Point) -> i32 {
        self.get_rectangle().winding(pt)
    }
    #[inline]
    fn bounding_box(&self) -> Rect {
        self.get_rectangle().bounding_box()
    }
    #[inline]
    fn as_rect(&self) -> Option<Rect> {
        self.get_rectangle().as_rect()
    }
    #[inline]
    fn contains(&self, pt: Point) -> bool {
        self.get_rectangle().abs().contains(pt)
    }
}

impl ObjectsFuncs for ShapeRectangle {
    const TOLERANCE: f64 = 0.01;
    const GRAB_RADIUS: f64 = 5.;
    type Kindvars = BSKindvars;

    fn save_vars(&mut self) {
        self.tl.saved_pos = self.tl.pos;
        self.br.saved_pos = self.br.pos;
    }
    fn restore_saved(&mut self) {
        self.tl.pos = self.tl.saved_pos;
        self.br.pos = self.br.saved_pos;
        self.update_polygon();
    }
    fn get_vars(&self) -> BSKindvars {
        BSKindvars::Rectangle(self.tl, self.br)
    }
    fn set_vars(&mut self, vars: &BSKindvars) {
        if let BSKindvars::Rectangle(tl, br) = vars {
            self.tl = tl.clone();
            self.br = br.clone();
            self.update_polygon();
        }
    }

    fn good_size(&self) -> bool {
        (self.tl.pos.x - self.br.pos.x).abs() >= ShapeRectangle::MIN_SIZE
            && (self.tl.pos.y - self.br.pos.y).abs() >= ShapeRectangle::MIN_SIZE
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
                let select = self.tl.selected
                    || self.tr.selected
                    || self.br.selected
                    || self.bl.selected
                    || self.top.selected
                    || self.right.selected
                    || self.bottom.selected
                    || self.left.selected;

                if select {
                    Some(self.get_position())
                } else {
                    None
                }
            }
            IsAnyModifierHighlighted => {
                let highlight = self.tl.highlighted
                    || self.tr.highlighted
                    || self.br.highlighted
                    || self.bl.highlighted
                    || self.top.highlighted
                    || self.right.highlighted
                    || self.bottom.highlighted
                    || self.left.highlighted;
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

    fn move_position(&mut self, mut dpos: Vec2, snap: f64) -> Option<Vec2> {
        dpos = snap_pt(dpos, snap);
        self.tl.pos = self.tl.saved_pos + dpos;
        self.br.pos = self.br.saved_pos + dpos;
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
        let tl_saved = self.tl.saved_pos;
        let br_saved = self.br.saved_pos;
        let tr_saved = self.get_tr_saved_modifier();
        let bl_saved = self.get_bl_saved_modifier();
        let top_saved = self.get_top_saved_modifier();
        let right_saved = self.get_right_saved_modifier();
        let bottom_saved = self.get_bottom_saved_modifier();
        let left_saved = self.get_left_saved_modifier();

        let tl_sel = self.tl.selected;
        let tr_sel = self.tr.selected;
        let br_sel = self.br.selected;
        let bl_sel = self.bl.selected;
        let top_sel = self.top.selected;
        let right_sel = self.right.selected;
        let bottom_sel = self.bottom.selected;
        let left_sel = self.left.selected;

        let dpos = pos - pos_init;
        const MIN_SIZE: f64 = ShapeRectangle::MIN_SIZE;

        match (tl_sel, tr_sel, br_sel, bl_sel) {
            (true, false, false, false) => {
                let mut tlpos = tl_saved + dpos;
                tlpos.x = tlpos.x.min(br_saved.x - MIN_SIZE);
                tlpos.y = tlpos.y.min(br_saved.y - MIN_SIZE);
                self.tl.pos = snap_pt(tlpos, snap);
                self.update_polygon();
                return Some(tlpos);
            }
            (false, true, false, false) => {
                let mut trpos = tr_saved + dpos;
                trpos.x = trpos.x.max(tl_saved.x + MIN_SIZE);
                trpos.y = trpos.y.min(br_saved.y - MIN_SIZE);
                self.br.pos = snap_pt(Vec2::new(trpos.x, br_saved.y), snap);
                self.tl.pos = snap_pt(Vec2::new(tl_saved.x, trpos.y), snap);
                self.update_polygon();
                return Some(trpos);
            }
            (false, false, true, false) => {
                let mut brpos = br_saved + dpos;
                brpos.x = brpos.x.max(bl_saved.x + MIN_SIZE);
                brpos.y = brpos.y.max(tr_saved.y + MIN_SIZE);
                self.br.pos = snap_pt(brpos, snap);
                self.update_polygon();
                return Some(brpos);
            }
            (false, false, false, true) => {
                let mut blpos = bl_saved + dpos;
                blpos.x = blpos.x.min(br_saved.x - MIN_SIZE);
                blpos.y = blpos.y.max(tl_saved.y + MIN_SIZE);
                self.tl.pos = snap_pt(Vec2::new(blpos.x, tl_saved.y), snap);
                self.br.pos = snap_pt(Vec2::new(br_saved.x, blpos.y), snap);
                self.update_polygon();
                return Some(blpos);
            }
            _ => (),
        };

        match (top_sel, right_sel, bottom_sel, left_sel) {
            (true, false, false, false) => {
                let mut toppos = top_saved + dpos;
                toppos.y = toppos.y.min(bottom_saved.y - MIN_SIZE);
                self.tl.pos = snap_pt(Vec2::new(tl_saved.x, toppos.y), snap);
                self.update_polygon();
                return Some(toppos);
            }
            (false, true, false, false) => {
                let mut rightpos = right_saved + dpos;
                rightpos.x = rightpos.x.max(left_saved.x + MIN_SIZE);
                self.br.pos = snap_pt(Vec2::new(rightpos.x, br_saved.y), snap);
                self.update_polygon();
                return Some(rightpos);
            }
            (false, false, true, false) => {
                let mut bottompos = bottom_saved + dpos;
                bottompos.y = bottompos.y.max(top_saved.y + MIN_SIZE);
                self.br.pos = snap_pt(Vec2::new(br_saved.x, bottompos.y), snap);
                self.update_polygon();
                return Some(bottompos);
            }
            (false, false, false, true) => {
                let mut leftpos = left_saved + dpos;
                leftpos.x = leftpos.x.min(right_saved.x - MIN_SIZE);
                self.tl.pos = snap_pt(Vec2::new(leftpos.x, tl_saved.y), snap);
                self.update_polygon();
                return Some(leftpos);
            }
            _ => (),
        };
        None
    }
    fn get_position(&self) -> Vec2 {
        (self.tl.pos + self.br.pos) / 2.
    }

    fn get_modifiers_paths(&self, _: &Size) -> Vec<(BezPath, Pattern)> {
        vec![
            (
                modifiers_path(self.tl.pos, 1., ShapeRectangle::GRAB_RADIUS),
                self.get_pattern_modifiers(self.tl.selected, self.tl.highlighted),
            ),
            (
                modifiers_path(self.get_tr_modifier(), 1., ShapeRectangle::GRAB_RADIUS),
                self.get_pattern_modifiers(self.tr.selected, self.tr.highlighted),
            ),
            (
                modifiers_path(self.br.pos, 1., ShapeRectangle::GRAB_RADIUS),
                self.get_pattern_modifiers(self.br.selected, self.br.highlighted),
            ),
            (
                modifiers_path(self.get_bl_modifier(), 1., ShapeRectangle::GRAB_RADIUS),
                self.get_pattern_modifiers(self.bl.selected, self.bl.highlighted),
            ),
            (
                modifiers_path(self.get_top_modifier(), 1., ShapeRectangle::GRAB_RADIUS),
                self.get_pattern_modifiers(self.top.selected, self.top.highlighted),
            ),
            (
                modifiers_path(self.get_right_modifier(), 1., ShapeRectangle::GRAB_RADIUS),
                self.get_pattern_modifiers(self.right.selected, self.right.highlighted),
            ),
            (
                modifiers_path(self.get_bottom_modifier(), 1., ShapeRectangle::GRAB_RADIUS),
                self.get_pattern_modifiers(self.bottom.selected, self.bottom.highlighted),
            ),
            (
                modifiers_path(self.get_left_modifier(), 1., ShapeRectangle::GRAB_RADIUS),
                self.get_pattern_modifiers(self.left.selected, self.left.highlighted),
            ),
            (
                center_path(
                    (self.tl.pos + self.br.pos) / 2.,
                    1.,
                    ShapeRectangle::GRAB_RADIUS,
                ),
                self.get_pattern_modifiers(self.selected, self.highlighted),
            ),
        ]
    }
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        let mut paths = vec![];
        let mut texts = vec![];
        let (path, text) = Dimension::new(
            DimKind::Horizontal,
            self.tl.pos,
            self.get_tr_modifier(),
            self.get_width(),
        )
        .get_path();
        paths.push(path);
        texts.push(text);

        let (path, text) = Dimension::new(
            DimKind::Vertical,
            self.get_bl_modifier(),
            self.tl.pos,
            self.get_height(),
        )
        .get_path();
        paths.push(path);
        texts.push(text);
        (paths, texts)
    }
    fn get_paths(&self, _: &Size) -> Vec<BezPath> {
        if self.good_size() {
            vec![self.get_rectangle().to_path(Self::TOLERANCE)]
        } else {
            vec![BezPath::new()]
        }
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

pub struct ShapeRectangleIter {
    rect_path_iter: RectPathIter,
}
impl Iterator for ShapeRectangleIter {
    type Item = PathEl;

    fn next(&mut self) -> Option<PathEl> {
        self.rect_path_iter.next()
    }
}
