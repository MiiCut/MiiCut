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
    Modifier,
};
use geo::{LineString, Polygon};
use kurbo::{
    BezPath, PathEl, Point, Rect, RoundedRect, RoundedRectPathIter, RoundedRectRadii, Shape, Size,
    Vec2,
};
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq)]
pub struct ShapeRectRounded {
    tl: Position,
    br: Position,
    radius_tl: Value,
    radius_tr: Value,
    radius_br: Value,
    radius_bl: Value,

    tr: Modifier,
    bl: Modifier,
    top: Modifier,
    right: Modifier,
    bottom: Modifier,
    left: Modifier,

    on_create: bool,
    highlighted: bool,
    selected: bool,

    segs: BezPath,
    polygon: Polygon<f64>,
}
impl ShapeRectRounded {
    const MIN_SIZE: f64 = 20.;
    const RAD_MIN_SIZE: f64 = ShapeRectRounded::MIN_SIZE / 2.;

    pub fn new(pos1: Vec2, pos2: Vec2) -> BSKind {
        let mut br = Position::new(pos2, true);
        br.selected = true;
        let tl = Position::new(pos1, true);
        BSKind::RectangleRounded(ShapeRectRounded {
            tl,
            br,
            radius_tl: Value::new(ShapeRectRounded::RAD_MIN_SIZE),
            radius_tr: Value::new(ShapeRectRounded::RAD_MIN_SIZE),
            radius_br: Value::new(ShapeRectRounded::RAD_MIN_SIZE),
            radius_bl: Value::new(ShapeRectRounded::RAD_MIN_SIZE),
            tr: Modifier::new(),
            bl: Modifier::new(),
            top: Modifier::new(),
            right: Modifier::new(),
            bottom: Modifier::new(),
            left: Modifier::new(),
            on_create: true,
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
    fn get_radii(&self) -> RoundedRectRadii {
        RoundedRectRadii {
            top_left: self.radius_tl.value,
            top_right: self.radius_tr.value,
            bottom_right: self.radius_br.value,
            bottom_left: self.radius_bl.value,
        }
    }

    fn get_rectangle_rounded(&self) -> RoundedRect {
        let (tl_pos, br_pos) = (self.tl.pos, self.br.pos);
        RoundedRect::new(tl_pos.x, tl_pos.y, br_pos.x, br_pos.y, self.get_radii())
    }

    fn _update_radii_from_outers(&mut self) {
        let tl = self.tl.pos;
        let tr = self.get_tr_modifier();
        let br = self.br.pos;
        let radius = (tl - tr).hypot().min((tr - br).hypot()) / 5.;

        self.radius_tl.value = radius;
        self.radius_tr.value = radius;
        self.radius_br.value = radius;
        self.radius_bl.value = radius;
    }

    fn get_tr_modifier(&self) -> Vec2 {
        Vec2::new(self.br.pos.x, self.tl.pos.y)
    }
    fn get_bl_modifier(&self) -> Vec2 {
        Vec2::new(self.tl.pos.x, self.br.pos.y)
    }

    fn get_rad_tl_modifier(&self) -> Vec2 {
        self.tl.pos + Vec2::new(self.radius_tl.value, self.radius_tl.value)
    }
    fn get_rad_tr_modifier(&self) -> Vec2 {
        self.get_tr_modifier() + Vec2::new(-self.radius_tr.value, self.radius_tr.value)
    }
    fn get_rad_br_modifier(&self) -> Vec2 {
        self.br.pos + Vec2::new(-self.radius_br.value, -self.radius_br.value)
    }
    fn get_rad_bl_modifier(&self) -> Vec2 {
        self.get_bl_modifier() + Vec2::new(self.radius_bl.value, -self.radius_bl.value)
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
        self.radius_tl.highlighted = value;
        self.radius_tr.highlighted = value;
        self.radius_br.highlighted = value;
        self.radius_bl.highlighted = value;
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
        self.radius_tl.selected = value;
        self.radius_tr.selected = value;
        self.radius_br.selected = value;
        self.radius_bl.selected = value;
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
        self.radius_tl.highlighted = (pos - self.get_rad_tl_modifier()).hypot() < grab;
        self.radius_tr.highlighted = (pos - self.get_rad_tr_modifier()).hypot() < grab;
        self.radius_br.highlighted = (pos - self.get_rad_br_modifier()).hypot() < grab;
        self.radius_bl.highlighted = (pos - self.get_rad_bl_modifier()).hypot() < grab;
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
        self.radius_tl.selected = (pos - self.get_rad_tl_modifier()).hypot() < grab;
        self.radius_tr.selected = (pos - self.get_rad_tr_modifier()).hypot() < grab;
        self.radius_br.selected = (pos - self.get_rad_br_modifier()).hypot() < grab;
        self.radius_bl.selected = (pos - self.get_rad_bl_modifier()).hypot() < grab;
    }
}
impl Display for ShapeRectRounded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rounded rectangle")
    }
}
impl Shape for ShapeRectRounded {
    type PathElementsIter<'iter> = ShapeRectRoundedIter;

    fn path_elements(&self, tolerance: f64) -> ShapeRectRoundedIter {
        ShapeRectRoundedIter {
            rouned_rect_path_iter: self.get_rectangle_rounded().path_elements(tolerance),
        }
    }
    #[inline]
    fn area(&self) -> f64 {
        self.get_rectangle_rounded().area()
    }
    #[inline]
    fn perimeter(&self, accuracy: f64) -> f64 {
        self.get_rectangle_rounded().perimeter(accuracy)
    }
    #[inline]
    fn winding(&self, pt: Point) -> i32 {
        self.get_rectangle_rounded().winding(pt)
    }
    #[inline]
    fn bounding_box(&self) -> Rect {
        self.get_rectangle_rounded().bounding_box()
    }
    #[inline]
    fn as_rounded_rect(&self) -> Option<RoundedRect> {
        self.get_rectangle_rounded().as_rounded_rect()
    }
    #[inline]
    fn contains(&self, pt: Point) -> bool {
        self.get_rectangle_rounded().contains(pt)
    }
}
impl ObjectsFuncs for ShapeRectRounded {
    const TOLERANCE: f64 = 0.01;
    const GRAB_RADIUS: f64 = 5.;
    type Kindvars = BSKindvars;

    fn save_vars(&mut self) {
        self.tl.saved_pos = self.tl.pos;
        self.br.saved_pos = self.br.pos;
        self.radius_tl.saved_val = self.radius_tl.value;
        self.radius_tr.saved_val = self.radius_tr.value;
        self.radius_br.saved_val = self.radius_br.value;
        self.radius_bl.saved_val = self.radius_bl.value;
    }
    fn restore_saved(&mut self) {
        self.tl.pos = self.tl.saved_pos;
        self.br.pos = self.br.saved_pos;
        self.radius_tl.value = self.radius_tl.saved_val;
        self.radius_tr.value = self.radius_tr.saved_val;
        self.radius_br.value = self.radius_br.saved_val;
        self.radius_bl.value = self.radius_bl.saved_val;
        self.update_polygon();
    }
    fn get_vars(&self) -> BSKindvars {
        BSKindvars::RectangleRounded(
            self.tl,
            self.br,
            self.radius_tl,
            self.radius_tr,
            self.radius_br,
            self.radius_bl,
        )
    }
    fn set_vars(&mut self, vars: &BSKindvars) {
        if let BSKindvars::RectangleRounded(tl, br, radius_tl, radius_tr, radius_br, radius_bl) =
            vars
        {
            self.tl = tl.clone();
            self.br = br.clone();
            self.radius_tl = radius_tl.clone();
            self.radius_tr = radius_tr.clone();
            self.radius_br = radius_br.clone();
            self.radius_bl = radius_bl.clone();
            self.update_polygon();
        }
    }
    fn good_size(&self) -> bool {
        ((self.tl.pos.x - self.br.pos.x).abs() >= ShapeRectRounded::MIN_SIZE)
            && ((self.tl.pos.y - self.br.pos.y).abs() >= ShapeRectRounded::MIN_SIZE)
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
                let select = self.tl.selected
                    || self.tr.selected
                    || self.br.selected
                    || self.bl.selected
                    || self.top.selected
                    || self.right.selected
                    || self.bottom.selected
                    || self.left.selected
                    || self.radius_tl.selected
                    || self.radius_tr.selected
                    || self.radius_br.selected
                    || self.radius_bl.selected;
                if select {
                    Some(self.get_position())
                } else {
                    None
                }
            }
            IsAnyModifierHighligh => {
                let highlight = self.tl.highlighted
                    || self.tr.highlighted
                    || self.br.highlighted
                    || self.bl.highlighted
                    || self.top.highlighted
                    || self.right.highlighted
                    || self.bottom.highlighted
                    || self.left.highlighted
                    || self.radius_tl.highlighted
                    || self.radius_tr.highlighted
                    || self.radius_br.highlighted
                    || self.radius_bl.highlighted;
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
            SetHighli(value) => self.highlighted = value,
            HighliFromPos(pos, ..) => {
                self.highlighted = self.contains(pos.to_point());
            }

            SelectAllModifiers(value) => self.select_all_modifiers(value),
            SelectModifierFromPos(pos, ..) => {
                self.select_modifiers_from_pos(pos, Self::GRAB_RADIUS);
            }

            HighliAllModifiers(value) => self.highlight_all_modifiers(value),
            HighliModifierFromPos(pos, ..) => {
                self.highlight_modifiers_from_pos(pos, Self::GRAB_RADIUS);
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
        const MIN_SIZE: f64 = ShapeRectRounded::MIN_SIZE;
        const RAD_MIN_SIZE: f64 = ShapeRectRounded::RAD_MIN_SIZE;

        let tl_saved = self.tl.saved_pos;
        let br_saved = self.br.saved_pos;
        let tr_saved = self.get_tr_saved_modifier();
        let bl_saved = self.get_bl_saved_modifier();
        let top_saved = self.get_top_saved_modifier();
        let right_saved = self.get_right_saved_modifier();
        let bottom_saved = self.get_bottom_saved_modifier();
        let left_saved = self.get_left_saved_modifier();
        let radius_tl_saved = self.radius_tl.saved_val;
        let radius_tr_saved = self.radius_tr.saved_val;
        let radius_br_saved = self.radius_br.saved_val;
        let radius_bl_saved = self.radius_bl.saved_val;

        let tl_sel = self.tl.selected;
        let tr_sel = self.tr.selected;
        let br_sel = self.br.selected;
        let bl_sel = self.bl.selected;
        let top_sel = self.top.selected;
        let right_sel = self.right.selected;
        let bottom_sel = self.bottom.selected;
        let left_sel = self.left.selected;
        let rad_tl_sel = self.radius_tl.selected;
        let rad_tr_sel = self.radius_tr.selected;
        let rad_br_sel = self.radius_br.selected;
        let rad_bl_sel = self.radius_bl.selected;

        let dpos = pos - pos_init;

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

        let max_radius_size = self.get_width().min(self.get_height()) / 2.;
        match (rad_tl_sel, rad_tr_sel, rad_br_sel, rad_bl_sel) {
            (true, false, false, false) => {
                let (proj, dir) = project_to_segment_with_direction(
                    self.tl.pos,
                    (self.tl.pos + self.get_tr_modifier()) / 2.,
                    dpos,
                );
                let radius_tl = (radius_tl_saved + proj * dir)
                    .max(RAD_MIN_SIZE)
                    .min(max_radius_size);
                self.radius_tl.value = snap_val(radius_tl, snap);
                self.update_polygon();
                return Some(self.tl.pos + Vec2::new(self.radius_tl.value, self.radius_tl.value));
            }
            (false, true, false, false) => {
                let (proj, dir) = project_to_segment_with_direction(
                    self.get_tr_modifier(),
                    (self.tl.pos + self.get_tr_modifier()) / 2.,
                    dpos,
                );
                let radius_tr = (radius_tr_saved + proj * dir)
                    .max(RAD_MIN_SIZE)
                    .min(max_radius_size);
                self.radius_tr.value = snap_val(radius_tr, snap);
                self.update_polygon();
                return Some(
                    self.get_tr_modifier() + Vec2::new(-self.radius_tr.value, self.radius_tr.value),
                );
            }
            (false, false, true, false) => {
                let (proj, dir) = project_to_segment_with_direction(
                    self.br.pos,
                    (self.br.pos + self.get_bl_modifier()) / 2.,
                    dpos,
                );
                let radius_br = (radius_br_saved + proj * dir)
                    .max(RAD_MIN_SIZE)
                    .min(max_radius_size);
                self.radius_br.value = snap_val(radius_br, snap);
                self.update_polygon();
                return Some(self.br.pos + Vec2::new(-self.radius_br.value, -self.radius_br.value));
            }
            (false, false, false, true) => {
                let (proj, dir) = project_to_segment_with_direction(
                    self.get_bl_modifier(),
                    (self.br.pos + self.get_bl_modifier()) / 2.,
                    dpos,
                );
                let radius_bl = (radius_bl_saved + proj * dir)
                    .max(RAD_MIN_SIZE)
                    .min(max_radius_size);
                self.radius_bl.value = snap_val(radius_bl, snap);
                self.update_polygon();
                return Some(
                    self.get_bl_modifier() + Vec2::new(self.radius_bl.value, -self.radius_bl.value),
                );
            }
            _ => (),
        };
        None
    }

    fn get_position(&self) -> Vec2 {
        (self.tl.pos + self.br.pos) / 2.
    }

    fn get_mod_paths_and_patterns(
        &self,
        _: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        let tl = self.tl.pos;
        let tr = self.get_tr_modifier();
        let br = self.br.pos;
        let bl = self.get_bl_modifier();

        let rad_tl_center = tl + Vec2::new(self.radius_tl.value, self.radius_tl.value);
        let rad_tr_center = tr + Vec2::new(-self.radius_tr.value, self.radius_tr.value);
        let rad_br_center = br + Vec2::new(-self.radius_br.value, -self.radius_br.value);
        let rad_bl_center = bl + Vec2::new(self.radius_bl.value, -self.radius_bl.value);
        let top = self.get_top_modifier();
        let right = self.get_right_modifier();
        let bottom = self.get_bottom_modifier();
        let left = self.get_left_modifier();

        vec![
            (
                modifiers_path(tl, 1., ShapeRectRounded::GRAB_RADIUS),
                self.get_pattern_status(self.tl.selected, self.tl.highlighted),
            ),
            (
                modifiers_path(tr, 1., ShapeRectRounded::GRAB_RADIUS),
                self.get_pattern_status(self.tr.selected, self.tr.highlighted),
            ),
            (
                modifiers_path(br, 1., ShapeRectRounded::GRAB_RADIUS),
                self.get_pattern_status(self.br.selected, self.br.highlighted),
            ),
            (
                modifiers_path(bl, 1., ShapeRectRounded::GRAB_RADIUS),
                self.get_pattern_status(self.bl.selected, self.bl.highlighted),
            ),
            (
                modifiers_path(top, 1., ShapeRectRounded::GRAB_RADIUS),
                self.get_pattern_status(self.top.selected, self.top.highlighted),
            ),
            (
                modifiers_path(right, 1., ShapeRectRounded::GRAB_RADIUS),
                self.get_pattern_status(self.right.selected, self.right.highlighted),
            ),
            (
                modifiers_path(bottom, 1., ShapeRectRounded::GRAB_RADIUS),
                self.get_pattern_status(self.bottom.selected, self.bottom.highlighted),
            ),
            (
                modifiers_path(left, 1., ShapeRectRounded::GRAB_RADIUS),
                self.get_pattern_status(self.left.selected, self.left.highlighted),
            ),
            (
                modifiers_path(rad_tl_center, 1., ShapeRectRounded::GRAB_RADIUS),
                self.get_pattern_status(self.radius_tl.selected, self.radius_tl.highlighted),
            ),
            (
                modifiers_path(rad_tr_center, 1., ShapeRectRounded::GRAB_RADIUS),
                self.get_pattern_status(self.radius_tr.selected, self.radius_tr.highlighted),
            ),
            (
                modifiers_path(rad_br_center, 1., ShapeRectRounded::GRAB_RADIUS),
                self.get_pattern_status(self.radius_br.selected, self.radius_br.highlighted),
            ),
            (
                modifiers_path(rad_bl_center, 1., ShapeRectRounded::GRAB_RADIUS),
                self.get_pattern_status(self.radius_bl.selected, self.radius_bl.highlighted),
            ),
            (
                center_path(
                    (self.tl.pos + self.br.pos) / 2.,
                    1.,
                    ShapeRectRounded::GRAB_RADIUS,
                ),
                self.get_pattern_status(self.selected, self.highlighted),
            ),
        ]
    }
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        let mut paths = vec![];
        let mut texts = vec![];

        let tl = self.tl.pos;
        let tr = self.get_tr_modifier();
        let br = self.br.pos;
        let bl = self.get_bl_modifier();
        let rad_tl_center = tl + Vec2::new(self.radius_tl.value, self.radius_tl.value);
        let rad_tr_center = tr + Vec2::new(-self.radius_tr.value, self.radius_tr.value);
        let rad_br_center = br + Vec2::new(-self.radius_br.value, -self.radius_br.value);
        let rad_bl_center = bl + Vec2::new(self.radius_bl.value, -self.radius_bl.value);

        let rad_tl = self.radius_tl.value / 2_f64.sqrt();
        let rad_tr = self.radius_tr.value / 2_f64.sqrt();
        let rad_br = self.radius_br.value / 2_f64.sqrt();
        let rad_bl = self.radius_bl.value / 2_f64.sqrt();

        let (path, text) = Dimension::new(DimKind::Horizontal, tl, tr, self.get_width()).get_path();
        paths.push(path);
        texts.push(text);

        let (path, text) = Dimension::new(DimKind::Vertical, bl, tl, self.get_height()).get_path();
        paths.push(path);
        texts.push(text);

        let (path, text) = Dimension::new(
            DimKind::Radius,
            rad_tl_center,
            rad_tl_center + Vec2::new(-rad_tl, -rad_tl),
            self.radius_tl.value,
        )
        .get_path();
        paths.push(path);
        texts.push(text);

        let (path, text) = Dimension::new(
            DimKind::Radius,
            rad_tr_center,
            rad_tr_center + Vec2::new(rad_tr, -rad_tr),
            self.radius_tr.value,
        )
        .get_path();
        paths.push(path);
        texts.push(text);

        let (path, text) = Dimension::new(
            DimKind::Radius,
            rad_br_center,
            rad_br_center + Vec2::new(rad_br, rad_br),
            self.radius_br.value,
        )
        .get_path();
        paths.push(path);
        texts.push(text);

        let (path, text) = Dimension::new(
            DimKind::Radius,
            rad_bl_center,
            rad_bl_center + Vec2::new(-rad_bl, rad_bl),
            self.radius_bl.value,
        )
        .get_path();
        paths.push(path);
        texts.push(text);

        (paths, texts)
    }
    fn get_paths(&self, _: &Size) -> Vec<BezPath> {
        if self.good_size() {
            vec![self.get_rectangle_rounded().to_path(Self::TOLERANCE)]
        } else {
            vec![BezPath::new()]
        }
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

pub struct ShapeRectRoundedIter {
    rouned_rect_path_iter: RoundedRectPathIter,
}
impl Iterator for ShapeRectRoundedIter {
    type Item = PathEl;

    fn next(&mut self) -> Option<PathEl> {
        self.rouned_rect_path_iter.next()
    }
}
