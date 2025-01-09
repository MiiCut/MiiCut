// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
use super::shapes::{ShapeKind, ShapeKindFuncs, ShapeKindvars};
use crate::{
    canvas::{CanvasText, Pattern},
    dimensions::{DimKind, Dimension},
    math::*,
    positions::{Position, Value, HS},
    prefab::{center_path, modifiers_path},
    Modifier,
};
use geo::{LineString, Polygon};
use kurbo::{
    BezPath, PathEl, Point, Rect, RoundedRect, RoundedRectPathIter, RoundedRectRadii, Shape, Vec2,
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

    pub fn new(pos1: Vec2, pos2: Vec2) -> ShapeKind {
        let mut br = Position::new(pos2, true);
        br.select(true);
        let tl = Position::new(pos1, true);
        ShapeKind::RectangleRounded(ShapeRectRounded {
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

    fn update_polygon(&mut self) {
        log!("calc rect rounded polygon");
        self.segs = calc_segs(self.get_paths());
        self.polygon = calc_polygon(&self.segs);
    }
    fn get_radii(&self) -> RoundedRectRadii {
        RoundedRectRadii {
            top_left: self.radius_tl.get_val(),
            top_right: self.radius_tr.get_val(),
            bottom_right: self.radius_br.get_val(),
            bottom_left: self.radius_bl.get_val(),
        }
    }
    fn get_width(&self) -> f64 {
        (self.tl.get_pos().x - self.br.get_pos().x).abs()
    }
    fn get_height(&self) -> f64 {
        (self.tl.get_pos().y - self.br.get_pos().y).abs()
    }

    fn get_rectangle_rounded(&self) -> RoundedRect {
        let (tl_pos, br_pos) = (self.tl.get_pos(), self.br.get_pos());
        RoundedRect::new(tl_pos.x, tl_pos.y, br_pos.x, br_pos.y, self.get_radii())
    }

    fn update_radii_from_outers(&mut self) {
        let tl = self.tl.get_pos();
        let tr = self.get_tr_modifier();
        let br = self.br.get_pos();
        let radius = (tl - tr).hypot().min((tr - br).hypot()) / 5.;

        self.radius_tl.set_val(radius);
        self.radius_tr.set_val(radius);
        self.radius_br.set_val(radius);
        self.radius_bl.set_val(radius);
    }

    fn get_tr_modifier(&self) -> Vec2 {
        Vec2::new(self.br.get_pos().x, self.tl.get_pos().y)
    }
    fn get_bl_modifier(&self) -> Vec2 {
        Vec2::new(self.tl.get_pos().x, self.br.get_pos().y)
    }

    fn get_top_modifier(&self) -> Vec2 {
        (self.tl.get_pos() + self.get_tr_modifier()) / 2.
    }
    fn get_right_modifier(&self) -> Vec2 {
        (self.get_tr_modifier() + self.br.get_pos()) / 2.
    }
    fn get_bottom_modifier(&self) -> Vec2 {
        (self.br.get_pos() + self.get_bl_modifier()) / 2.
    }
    fn get_left_modifier(&self) -> Vec2 {
        (self.get_bl_modifier() + self.tl.get_pos()) / 2.
    }

    fn get_tr_saved_modifier(&self) -> Vec2 {
        Vec2::new(self.br.get_saved_pos().x, self.tl.get_saved_pos().y)
    }
    fn get_bl_saved_modifier(&self) -> Vec2 {
        Vec2::new(self.tl.get_saved_pos().x, self.br.get_saved_pos().y)
    }

    fn get_top_saved_modifier(&self) -> Vec2 {
        (self.tl.get_saved_pos() + self.get_tr_saved_modifier()) / 2.
    }
    fn get_right_saved_modifier(&self) -> Vec2 {
        (self.get_tr_saved_modifier() + self.br.get_saved_pos()) / 2.
    }
    fn get_bottom_saved_modifier(&self) -> Vec2 {
        (self.br.get_saved_pos() + self.get_bl_saved_modifier()) / 2.
    }
    fn get_left_saved_modifier(&self) -> Vec2 {
        (self.get_bl_saved_modifier() + self.tl.get_saved_pos()) / 2.
    }
}
impl Display for ShapeRectRounded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rounded rectangle")
    }
}
impl Shape for ShapeRectRounded {
    type PathElementsIter<'iter> = CShapeRectRoundedIter;

    fn path_elements(&self, tolerance: f64) -> CShapeRectRoundedIter {
        CShapeRectRoundedIter {
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

impl ShapeKindFuncs for ShapeRectRounded {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 2.;

    fn save_vars(&mut self) {
        self.tl.save_pos();
        self.br.save_pos();
        self.radius_tl.save_val();
        self.radius_tr.save_val();
        self.radius_br.save_val();
        self.radius_bl.save_val();
    }
    fn restore_saved(&mut self) {
        self.tl.restore_saved();
        self.br.restore_saved();
        self.radius_tl.restore_saved();
        self.radius_tr.restore_saved();
        self.radius_br.restore_saved();
        self.radius_bl.restore_saved();
        self.update_polygon();
    }
    fn get_vars(&self) -> ShapeKindvars {
        ShapeKindvars::RectangleRounded(
            self.tl,
            self.br,
            self.radius_tl,
            self.radius_tr,
            self.radius_br,
            self.radius_bl,
        )
    }
    fn set_vars(&mut self, vars: &ShapeKindvars) {
        if let ShapeKindvars::RectangleRounded(tl, br, radius_tl, radius_tr, radius_br, radius_bl) =
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
        ((self.tl.get_pos().x - self.br.get_pos().x).abs() >= ShapeRectRounded::MIN_SIZE)
            && ((self.tl.get_pos().y - self.br.get_pos().y).abs() >= ShapeRectRounded::MIN_SIZE)
    }

    fn set_hs_from_pos(&mut self, pos: Vec2, hors: HS) -> bool {
        match hors {
            HS::Highlight => {
                self.highlighted = self.contains(pos.to_point());
                self.highlighted
            }
            HS::Select => {
                self.selected = self.contains(pos.to_point());
                self.selected
            }
        }
    }
    fn set_hs(&mut self, value: bool, hors: HS) {
        match hors {
            HS::Highlight => self.highlighted = value,
            HS::Select => self.selected = value,
        }
    }
    fn get_hs(&self, hors: HS) -> bool {
        match hors {
            HS::Highlight => self.highlighted,
            HS::Select => self.selected,
        }
    }
    fn get_hhss(&self) -> (bool, bool) {
        (self.selected, self.highlighted)
    }

    fn set_hs_modifiers_from_pos(&mut self, pos: Vec2, hors: HS) -> bool {
        self.on_create = false;
        let tl_hors = (pos - self.tl.get_pos()).hypot() < Self::GRAB;
        let tr_hors = (pos - self.get_tr_modifier()).hypot() < Self::GRAB;
        let br_hors = (pos - self.br.get_pos()).hypot() < Self::GRAB;
        let bl_hors = (pos - self.get_bl_modifier()).hypot() < Self::GRAB;
        let rad_tl_hors = (pos
            - (self.tl.get_pos() + Vec2::new(self.radius_tl.get_val(), self.radius_tl.get_val())))
        .hypot()
            < Self::GRAB;
        let rad_tr_hors = (pos
            - (self.get_tr_modifier()
                + Vec2::new(-self.radius_tr.get_val(), self.radius_tr.get_val())))
        .hypot()
            < Self::GRAB;
        let rad_br_hors = (pos
            - (self.br.get_pos()
                + Vec2::new(-self.radius_br.get_val(), -self.radius_br.get_val())))
        .hypot()
            < Self::GRAB;
        let rad_bl_hors = (pos
            - (self.get_bl_modifier()
                + Vec2::new(self.radius_bl.get_val(), -self.radius_bl.get_val())))
        .hypot()
            < Self::GRAB;
        let top_hors = (pos - self.get_top_modifier()).hypot() < Self::GRAB;
        let right_hors = (pos - self.get_right_modifier()).hypot() < Self::GRAB;
        let bottom_hors = (pos - self.get_bottom_modifier()).hypot() < Self::GRAB;
        let left_hors = (pos - self.get_left_modifier()).hypot() < Self::GRAB;

        match hors {
            HS::Highlight => {
                self.tl.highlight(tl_hors);
                self.tr.highlight(tr_hors);
                self.br.highlight(br_hors);
                self.bl.highlight(bl_hors);
                self.radius_tl.highlight(rad_tl_hors);
                self.radius_tr.highlight(rad_tr_hors);
                self.radius_br.highlight(rad_br_hors);
                self.radius_bl.highlight(rad_bl_hors);

                self.top.highlight(top_hors);
                self.right.highlight(right_hors);
                self.bottom.highlight(bottom_hors);
                self.left.highlight(left_hors);
                self.tl.is_highlighted()
                    || self.tr.is_highlighted()
                    || self.br.is_highlighted()
                    || self.bl.is_highlighted()
                    || self.radius_tl.is_highlighted()
                    || self.radius_tr.is_highlighted()
                    || self.radius_br.is_highlighted()
                    || self.radius_bl.is_highlighted()
                    || self.top.is_highlighted()
                    || self.right.is_highlighted()
                    || self.bottom.is_highlighted()
                    || self.left.is_highlighted()
            }
            HS::Select => {
                log!("set_hors_modifiers_from_pos");
                self.tl.select(tl_hors);
                self.tr.select(tr_hors);
                self.br.select(br_hors);
                self.bl.select(bl_hors);
                self.radius_tl.select(rad_tl_hors);
                self.radius_tr.select(rad_tr_hors);
                self.radius_br.select(rad_br_hors);
                self.radius_bl.select(rad_bl_hors);
                self.top.select(top_hors);
                self.right.select(right_hors);
                self.bottom.select(bottom_hors);
                self.left.select(left_hors);
                self.tl.is_selected()
                    || self.tr.is_selected()
                    || self.br.is_selected()
                    || self.bl.is_selected()
                    || self.radius_tl.is_selected()
                    || self.radius_tr.is_selected()
                    || self.radius_br.is_selected()
                    || self.radius_bl.is_selected()
                    || self.top.is_selected()
                    || self.right.is_selected()
                    || self.bottom.is_selected()
                    || self.left.is_selected()
            }
        }
    }
    fn set_hs_modifiers(&mut self, value: bool, hors: HS) {
        match hors {
            HS::Highlight => {
                self.tl.highlight(value);
                self.tr.highlight(value);
                self.br.highlight(value);
                self.bl.highlight(value);
                self.top.highlight(value);
                self.right.highlight(value);
                self.bottom.highlight(value);
                self.left.highlight(value);
                self.radius_tl.highlight(value);
                self.radius_tr.highlight(value);
                self.radius_br.highlight(value);
                self.radius_bl.highlight(value);
            }
            HS::Select => {
                self.tl.select(value);
                self.tr.select(value);
                self.br.select(value);
                self.bl.select(value);
                self.top.select(value);
                self.right.select(value);
                self.bottom.select(value);
                self.left.select(value);
                self.radius_tl.select(value);
                self.radius_tr.select(value);
                self.radius_br.select(value);
                self.radius_bl.select(value);
            }
        }
    }
    fn get_hs_modifiers(&self, hors: HS) -> bool {
        match hors {
            HS::Highlight => {
                self.tl.is_highlighted()
                    || self.tr.is_highlighted()
                    || self.br.is_highlighted()
                    || self.bl.is_highlighted()
                    || self.top.is_highlighted()
                    || self.right.is_highlighted()
                    || self.bottom.is_highlighted()
                    || self.left.is_highlighted()
                    || self.radius_tl.is_highlighted()
                    || self.radius_tr.is_highlighted()
                    || self.radius_br.is_highlighted()
                    || self.radius_bl.is_highlighted()
            }
            HS::Select => {
                self.tl.is_selected()
                    || self.tr.is_selected()
                    || self.br.is_selected()
                    || self.bl.is_selected()
                    || self.top.is_selected()
                    || self.right.is_selected()
                    || self.bottom.is_selected()
                    || self.left.is_selected()
                    || self.radius_tl.is_selected()
                    || self.radius_tr.is_selected()
                    || self.radius_br.is_selected()
                    || self.radius_bl.is_selected()
            }
        }
    }

    fn toggle_prop(&mut self) {
        ()
    }

    fn move_position(&mut self, dpos: Vec2) {
        self.tl.set_pos(self.tl.get_saved_pos() + dpos);
        self.br.set_pos(self.br.get_saved_pos() + dpos);
        self.update_polygon();
    }
    fn move_modifier(&mut self, pos_init: Vec2, pos: Vec2, _shift_pressed: bool) -> bool {
        const MIN_SIZE: f64 = ShapeRectRounded::MIN_SIZE;
        const RAD_MIN_SIZE: f64 = ShapeRectRounded::RAD_MIN_SIZE;

        let tl_saved = self.tl.get_saved_pos();
        let br_saved = self.br.get_saved_pos();
        let tr_saved = self.get_tr_saved_modifier();
        let bl_saved = self.get_bl_saved_modifier();
        let top_saved = self.get_top_saved_modifier();
        let right_saved = self.get_right_saved_modifier();
        let bottom_saved = self.get_bottom_saved_modifier();
        let left_saved = self.get_left_saved_modifier();
        let radius_tl_saved = self.radius_tl.get_saved_val();
        let radius_tr_saved = self.radius_tr.get_saved_val();
        let radius_br_saved = self.radius_br.get_saved_val();
        let radius_bl_saved = self.radius_bl.get_saved_val();

        let tl_sel = self.tl.is_selected();
        let tr_sel = self.tr.is_selected();
        let br_sel = self.br.is_selected();
        let bl_sel = self.bl.is_selected();
        let top_sel = self.top.is_selected();
        let right_sel = self.right.is_selected();
        let bottom_sel = self.bottom.is_selected();
        let left_sel = self.left.is_selected();
        let rad_tl_sel = self.radius_tl.is_selected();
        let rad_tr_sel = self.radius_tr.is_selected();
        let rad_br_sel = self.radius_br.is_selected();
        let rad_bl_sel = self.radius_bl.is_selected();

        let dpos = pos - pos_init;

        let modified = match (tl_sel, tr_sel, br_sel, bl_sel) {
            (true, false, false, false) => {
                let mut tlpos = tl_saved + dpos;
                tlpos.x = tlpos.x.min(tr_saved.x - MIN_SIZE);
                tlpos.y = tlpos.y.min(bl_saved.y - MIN_SIZE);
                self.tl.set_pos(tlpos);
                true
            }
            (false, true, false, false) => {
                let mut trpos = tr_saved + dpos;
                trpos.x = trpos.x.max(tl_saved.x + MIN_SIZE);
                trpos.y = trpos.y.min(br_saved.y - MIN_SIZE);
                self.br.set_pos(Vec2::new(trpos.x, br_saved.y));
                self.tl.set_pos(Vec2::new(tl_saved.x, trpos.y));
                true
            }
            (false, false, true, false) => {
                let mut brpos = br_saved + dpos;
                brpos.x = brpos.x.max(bl_saved.x + MIN_SIZE);
                brpos.y = brpos.y.max(tr_saved.y + MIN_SIZE);
                self.br.set_pos(brpos);
                true
            }
            (false, false, false, true) => {
                let mut blpos = bl_saved + dpos;
                blpos.x = blpos.x.min(br_saved.x - MIN_SIZE);
                blpos.y = blpos.y.max(tl_saved.y + MIN_SIZE);
                self.tl.set_pos(Vec2::new(blpos.x, tl_saved.y));
                self.br.set_pos(Vec2::new(br_saved.x, blpos.y));
                true
            }
            _ => false,
        };

        if modified {
            if self.on_create && br_sel {
                self.update_radii_from_outers();
            }
            self.update_polygon();
            return true;
        }

        let modified = match (top_sel, right_sel, bottom_sel, left_sel) {
            (true, false, false, false) => {
                let mut toppos = top_saved + dpos;
                toppos.y = toppos.y.min(bottom_saved.y - MIN_SIZE);
                self.tl.set_pos(Vec2::new(tl_saved.x, toppos.y));
                true
            }
            (false, true, false, false) => {
                let mut rightpos = right_saved + dpos;
                rightpos.x = rightpos.x.max(left_saved.x + MIN_SIZE);
                self.br.set_pos(Vec2::new(rightpos.x, br_saved.y));
                true
            }
            (false, false, true, false) => {
                let mut bottompos = bottom_saved + dpos;
                bottompos.y = bottompos.y.max(top_saved.y + MIN_SIZE);
                self.br.set_pos(Vec2::new(br_saved.x, bottompos.y));
                true
            }
            (false, false, false, true) => {
                let mut leftpos = left_saved + dpos;
                leftpos.x = leftpos.x.min(right_saved.x - MIN_SIZE);
                self.tl.set_pos(Vec2::new(leftpos.x, tl_saved.y));
                true
            }
            _ => false,
        };
        if modified {
            self.update_polygon();
            return true;
        }

        let max_radius_size = self.get_width().min(self.get_height()) / 2.;
        log!("max_radius_size: {}", max_radius_size);
        let modified = match (rad_tl_sel, rad_tr_sel, rad_br_sel, rad_bl_sel) {
            (true, false, false, false) => {
                let (proj, dir) = project_to_segment_with_direction(
                    self.tl.get_pos(),
                    (self.tl.get_pos() + self.get_tr_modifier()) / 2.,
                    dpos,
                );
                self.radius_tl.set_val(
                    (radius_tl_saved + proj * dir)
                        .max(RAD_MIN_SIZE)
                        .min(max_radius_size),
                );
                true
            }
            (false, true, false, false) => {
                let (proj, dir) = project_to_segment_with_direction(
                    self.get_tr_modifier(),
                    (self.tl.get_pos() + self.get_tr_modifier()) / 2.,
                    dpos,
                );
                self.radius_tr.set_val(
                    (radius_tr_saved + proj * dir)
                        .max(RAD_MIN_SIZE)
                        .min(max_radius_size),
                );
                true
            }
            (false, false, true, false) => {
                let (proj, dir) = project_to_segment_with_direction(
                    self.br.get_pos(),
                    (self.br.get_pos() + self.get_bl_modifier()) / 2.,
                    dpos,
                );
                self.radius_br.set_val(
                    (radius_br_saved + proj * dir)
                        .max(RAD_MIN_SIZE)
                        .min(max_radius_size),
                );
                true
            }
            (false, false, false, true) => {
                let (proj, dir) = project_to_segment_with_direction(
                    self.get_bl_modifier(),
                    (self.br.get_pos() + self.get_bl_modifier()) / 2.,
                    dpos,
                );
                self.radius_bl.set_val(
                    (radius_bl_saved + proj * dir)
                        .max(RAD_MIN_SIZE)
                        .min(max_radius_size),
                );
                true
            }
            _ => false,
        };
        if modified {
            self.update_polygon();
            return true;
        }
        false
    }
    fn get_position(&self) -> Vec2 {
        (self.tl.get_pos() + self.br.get_pos()) / 2.
    }

    fn get_modifiers_paths(&self) -> Vec<(BezPath, Pattern)> {
        let tl = self.tl.get_pos();
        let tr = self.get_tr_modifier();
        let br = self.br.get_pos();
        let bl = self.get_bl_modifier();

        let rad_tl_center = tl + Vec2::new(self.radius_tl.get_val(), self.radius_tl.get_val());
        let rad_tr_center = tr + Vec2::new(-self.radius_tr.get_val(), self.radius_tr.get_val());
        let rad_br_center = br + Vec2::new(-self.radius_br.get_val(), -self.radius_br.get_val());
        let rad_bl_center = bl + Vec2::new(self.radius_bl.get_val(), -self.radius_bl.get_val());
        let top = self.get_top_modifier();
        let right = self.get_right_modifier();
        let bottom = self.get_bottom_modifier();
        let left = self.get_left_modifier();

        vec![
            (
                modifiers_path(tl, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(self.tl.is_selected(), self.tl.is_highlighted()),
            ),
            (
                modifiers_path(tr, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(self.tr.is_selected(), self.tr.is_highlighted()),
            ),
            (
                modifiers_path(br, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(self.br.is_selected(), self.br.is_highlighted()),
            ),
            (
                modifiers_path(bl, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(self.bl.is_selected(), self.bl.is_highlighted()),
            ),
            (
                modifiers_path(top, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(self.top.is_selected(), self.top.is_highlighted()),
            ),
            (
                modifiers_path(right, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(self.right.is_selected(), self.right.is_highlighted()),
            ),
            (
                modifiers_path(bottom, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(self.bottom.is_selected(), self.bottom.is_highlighted()),
            ),
            (
                modifiers_path(left, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(self.left.is_selected(), self.left.is_highlighted()),
            ),
            (
                modifiers_path(rad_tl_center, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(
                    self.radius_tl.is_selected(),
                    self.radius_tl.is_highlighted(),
                ),
            ),
            (
                modifiers_path(rad_tr_center, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(
                    self.radius_tr.is_selected(),
                    self.radius_tr.is_highlighted(),
                ),
            ),
            (
                modifiers_path(rad_br_center, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(
                    self.radius_br.is_selected(),
                    self.radius_br.is_highlighted(),
                ),
            ),
            (
                modifiers_path(rad_bl_center, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(
                    self.radius_bl.is_selected(),
                    self.radius_bl.is_highlighted(),
                ),
            ),
            (
                center_path(
                    (self.tl.get_pos() + self.br.get_pos()) / 2.,
                    1.,
                    ShapeRectRounded::GRAB,
                ),
                self.get_pattern_modifiers(self.selected, self.highlighted),
            ),
        ]
    }
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        let mut paths = vec![];
        let mut texts = vec![];

        let tl = self.tl.get_pos();
        let tr = self.get_tr_modifier();
        let br = self.br.get_pos();
        let bl = self.get_bl_modifier();
        let rad_tl_center = tl + Vec2::new(self.radius_tl.get_val(), self.radius_tl.get_val());
        let rad_tr_center = tr + Vec2::new(-self.radius_tr.get_val(), self.radius_tr.get_val());
        let rad_br_center = br + Vec2::new(-self.radius_br.get_val(), -self.radius_br.get_val());
        let rad_bl_center = bl + Vec2::new(self.radius_bl.get_val(), -self.radius_bl.get_val());

        let rad_tl = self.radius_tl.get_val() / 2_f64.sqrt();
        let rad_tr = self.radius_tr.get_val() / 2_f64.sqrt();
        let rad_br = self.radius_br.get_val() / 2_f64.sqrt();
        let rad_bl = self.radius_bl.get_val() / 2_f64.sqrt();

        let (path, text) = Dimension::new(DimKind::Horizontal, tl, tr).get_path();
        paths.push(path);
        texts.push(text);

        let (path, text) = Dimension::new(DimKind::Vertical, bl, tl).get_path();
        paths.push(path);
        texts.push(text);

        let (path, text) = Dimension::new(
            DimKind::Radius,
            rad_tl_center,
            rad_tl_center + Vec2::new(-rad_tl, -rad_tl),
        )
        .get_path();
        paths.push(path);
        texts.push(text);

        let (path, text) = Dimension::new(
            DimKind::Radius,
            rad_tr_center,
            rad_tr_center + Vec2::new(rad_tr, -rad_tr),
        )
        .get_path();
        paths.push(path);
        texts.push(text);

        let (path, text) = Dimension::new(
            DimKind::Radius,
            rad_br_center,
            rad_br_center + Vec2::new(rad_br, rad_br),
        )
        .get_path();
        paths.push(path);
        texts.push(text);

        let (path, text) = Dimension::new(
            DimKind::Radius,
            rad_bl_center,
            rad_bl_center + Vec2::new(-rad_bl, rad_bl),
        )
        .get_path();
        paths.push(path);
        texts.push(text);

        (paths, texts)
    }
    fn get_paths(&self) -> Vec<BezPath> {
        if self.good_size() {
            vec![self.get_rectangle_rounded().to_path(Self::TOLERANCE)]
        } else {
            vec![BezPath::new()]
        }
    }
    fn get_polygon(&self) -> Polygon<f64> {
        self.polygon.clone()
    }
}

pub struct CShapeRectRoundedIter {
    rouned_rect_path_iter: RoundedRectPathIter,
}
impl Iterator for CShapeRectRoundedIter {
    type Item = PathEl;

    fn next(&mut self) -> Option<PathEl> {
        self.rouned_rect_path_iter.next()
    }
}
