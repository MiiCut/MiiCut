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
    KeysStates, Modifier, Pointer,
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
        self.segs = calc_segs(self.to_path(Self::TOLERANCE));
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

    fn highlight_modifiers_from_pos(&mut self, pointer: &mut Pointer, grab: f64) {
        if (pointer.pos() - self.tl.pos).hypot() < grab {
            self.tl.highlighted = true;
            pointer.set_pos(self.tl.pos);
        } else {
            self.tl.highlighted = false;
        }
        if (pointer.pos() - self.get_tr_modifier()).hypot() < grab {
            self.tr.highlighted = true;
            pointer.set_pos(self.get_tr_modifier());
        } else {
            self.tr.highlighted = false;
        }
        if (pointer.pos() - self.br.pos).hypot() < grab {
            self.br.highlighted = true;
            pointer.set_pos(self.br.pos);
        } else {
            self.br.highlighted = false;
        }
        if (pointer.pos() - self.get_bl_modifier()).hypot() < grab {
            self.bl.highlighted = true;
            pointer.set_pos(self.get_bl_modifier());
        } else {
            self.bl.highlighted = false;
        }
        if (pointer.pos() - self.get_top_modifier()).hypot() < grab {
            self.top.highlighted = true;
            pointer.set_pos(self.get_top_modifier());
        } else {
            self.top.highlighted = false;
        }
        if (pointer.pos() - self.get_right_modifier()).hypot() < grab {
            self.right.highlighted = true;
            pointer.set_pos(self.get_right_modifier());
        } else {
            self.right.highlighted = false;
        }
        if (pointer.pos() - self.get_bottom_modifier()).hypot() < grab {
            self.bottom.highlighted = true;
            pointer.set_pos(self.get_bottom_modifier());
        } else {
            self.bottom.highlighted = false;
        }
        if (pointer.pos() - self.get_left_modifier()).hypot() < grab {
            self.left.highlighted = true;
            pointer.set_pos(self.get_left_modifier());
        } else {
            self.left.highlighted = false;
        }
    }
    fn select_modifiers_from_pos(&mut self, pointer: &mut Pointer, grab: f64) {
        if (pointer.pos() - self.tl.pos).hypot() < grab {
            self.tl.selected = true;
            pointer.set_pos(self.tl.pos);
            pointer.save_pos();
        } else {
            self.tl.selected = false;
        }
        if (pointer.pos() - self.get_tr_modifier()).hypot() < grab {
            self.tr.selected = true;
            pointer.set_pos(self.get_tr_modifier());
            pointer.save_pos();
        } else {
            self.tr.selected = false;
        }
        if (pointer.pos() - self.br.pos).hypot() < grab {
            self.br.selected = true;
            pointer.set_pos(self.br.pos);
            pointer.save_pos();
        } else {
            self.br.selected = false;
        }
        if (pointer.pos() - self.get_bl_modifier()).hypot() < grab {
            self.bl.selected = true;
            pointer.set_pos(self.get_bl_modifier());
            pointer.save_pos();
        } else {
            self.bl.selected = false;
        }
        if (pointer.pos() - self.get_top_modifier()).hypot() < grab {
            self.top.selected = true;
            pointer.set_pos(self.get_top_modifier());
            pointer.save_pos();
        } else {
            self.top.selected = false;
        }
        if (pointer.pos() - self.get_right_modifier()).hypot() < grab {
            self.right.selected = true;
            pointer.set_pos(self.get_right_modifier());
            pointer.save_pos();
        } else {
            self.right.selected = false;
        }
        if (pointer.pos() - self.get_bottom_modifier()).hypot() < grab {
            self.bottom.selected = true;
            pointer.set_pos(self.get_bottom_modifier());
            pointer.save_pos();
        } else {
            self.bottom.selected = false;
        }
        if (pointer.pos() - self.get_left_modifier()).hypot() < grab {
            self.left.selected = true;
            pointer.set_pos(self.get_left_modifier());
            pointer.save_pos();
        } else {
            self.left.selected = false;
        }
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
                    || self.left.selected;

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
                self.select_modifiers_from_pos(pointer, Self::GRAB_RADIUS);
            }
            HighliModifierFromPos => {
                self.highlight_modifiers_from_pos(pointer, Self::GRAB_RADIUS);
            }
        }
    }

    fn toggle_prop(&mut self) {
        ()
    }

    fn move_position(&mut self, pointer: &mut Pointer, _keys_states: KeysStates) -> bool {
        let dpos = pointer.dpos();
        self.tl.pos = self.tl.saved_pos + dpos;
        self.br.pos = self.br.saved_pos + dpos;
        self.update_polygon();
        true
    }
    fn move_modifier(&mut self, pointer: &Pointer, _keys_states: KeysStates) -> bool {
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

        let dpos = pointer.dpos();
        let snap = pointer.get_snap().val();

        match (tl_sel, tr_sel, br_sel, bl_sel) {
            (true, false, false, false) => {
                self.tl.pos = if pointer.is_magnetized() {
                    tl_saved + dpos
                } else {
                    br_saved + snap_pt(tl_saved - br_saved + dpos, snap)
                };
                if self.br.pos.x - self.tl.pos.x < Self::MIN_SIZE {
                    self.tl.pos.x = self.br.pos.x - Self::MIN_SIZE;
                }
                if self.br.pos.y - self.tl.pos.y < Self::MIN_SIZE {
                    self.tl.pos.y = self.br.pos.y - Self::MIN_SIZE;
                }
                self.update_polygon();
                return true;
            }
            (false, true, false, false) => {
                let tr_pos = if pointer.is_magnetized() {
                    tr_saved + dpos
                } else {
                    Vec2::new(
                        tl_saved.x + snap_val(tr_saved.x - tl_saved.x + dpos.x, snap),
                        br_saved.y + snap_val(tr_saved.y - br_saved.y + dpos.y, snap),
                    )
                };
                self.br.pos.x = tr_pos.x;
                self.tl.pos.y = tr_pos.y;

                if self.br.pos.x - self.tl.pos.x < Self::MIN_SIZE {
                    self.br.pos.x = self.tl.pos.x + Self::MIN_SIZE;
                }
                if self.br.pos.y - self.tl.pos.y < Self::MIN_SIZE {
                    self.tl.pos.y = self.br.pos.y - Self::MIN_SIZE;
                }
                self.update_polygon();
                return true;
            }
            (false, false, true, false) => {
                self.br.pos = if pointer.is_magnetized() {
                    br_saved + dpos
                } else {
                    tl_saved + snap_pt(br_saved - tl_saved + dpos, snap)
                };

                if self.br.pos.x - self.tl.pos.x < Self::MIN_SIZE {
                    self.br.pos.x = self.tl.pos.x + Self::MIN_SIZE;
                }
                if self.br.pos.y - self.tl.pos.y < Self::MIN_SIZE {
                    self.br.pos.y = self.tl.pos.y + Self::MIN_SIZE;
                }
                self.update_polygon();
                return true;
            }
            (false, false, false, true) => {
                let bl_pos = if pointer.is_magnetized() {
                    bl_saved + dpos
                } else {
                    Vec2::new(
                        br_saved.x + snap_val(bl_saved.x - br_saved.x + dpos.x, snap),
                        tl_saved.y + snap_val(bl_saved.y - tl_saved.y + dpos.y, snap),
                    )
                };
                self.tl.pos.x = bl_pos.x;
                self.br.pos.y = bl_pos.y;

                if self.br.pos.x - self.tl.pos.x < Self::MIN_SIZE {
                    self.tl.pos.x = self.br.pos.x - Self::MIN_SIZE;
                }
                if self.br.pos.y - self.tl.pos.y < Self::MIN_SIZE {
                    self.br.pos.y = self.tl.pos.y + Self::MIN_SIZE;
                }
                self.update_polygon();
                return true;
            }
            _ => (),
        };

        match (top_sel, right_sel, bottom_sel, left_sel) {
            (true, false, false, false) => {
                let mut toppos = if pointer.is_magnetized() {
                    top_saved + dpos
                } else {
                    snap_pt(top_saved - bottom_saved + dpos, snap) + bottom_saved
                };
                toppos.y = toppos.y.min(bottom_saved.y - Self::MIN_SIZE);
                self.tl.pos.y = toppos.y;
                self.update_polygon();
                return true;
            }
            (false, true, false, false) => {
                let mut rightpos = if pointer.is_magnetized() {
                    right_saved + dpos
                } else {
                    snap_pt(right_saved - left_saved + dpos, snap) + left_saved
                };
                rightpos.x = rightpos.x.max(left_saved.x + Self::MIN_SIZE);
                self.br.pos.x = rightpos.x;
                self.update_polygon();
                return true;
            }
            (false, false, true, false) => {
                let mut bottompos = if pointer.is_magnetized() {
                    bottom_saved + dpos
                } else {
                    snap_pt(bottom_saved - top_saved + dpos, snap) + top_saved
                };
                bottompos.y = bottompos.y.max(top_saved.y + Self::MIN_SIZE);
                self.br.pos.y = bottompos.y;
                self.update_polygon();
                return true;
            }
            (false, false, false, true) => {
                let mut leftpos = if pointer.is_magnetized() {
                    left_saved + dpos
                } else {
                    snap_pt(left_saved - right_saved + dpos, snap) + right_saved
                };
                leftpos.x = leftpos.x.min(right_saved.x - Self::MIN_SIZE);
                self.tl.pos.x = leftpos.x;
                self.update_polygon();
                return true;
            }
            _ => (),
        };
        false
    }
    fn get_position(&self) -> Vec2 {
        (self.tl.pos + self.br.pos) / 2.
    }

    fn get_mod_paths_and_patterns(
        &self,
        _: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        vec![
            (
                modifiers_path(self.tl.pos, 1., ShapeRectangle::GRAB_RADIUS),
                self.get_pattern_status(self.tl.selected, self.tl.highlighted),
            ),
            (
                modifiers_path(self.get_tr_modifier(), 1., ShapeRectangle::GRAB_RADIUS),
                self.get_pattern_status(self.tr.selected, self.tr.highlighted),
            ),
            (
                modifiers_path(self.br.pos, 1., ShapeRectangle::GRAB_RADIUS),
                self.get_pattern_status(self.br.selected, self.br.highlighted),
            ),
            (
                modifiers_path(self.get_bl_modifier(), 1., ShapeRectangle::GRAB_RADIUS),
                self.get_pattern_status(self.bl.selected, self.bl.highlighted),
            ),
            (
                modifiers_path(self.get_top_modifier(), 1., ShapeRectangle::GRAB_RADIUS),
                self.get_pattern_status(self.top.selected, self.top.highlighted),
            ),
            (
                modifiers_path(self.get_right_modifier(), 1., ShapeRectangle::GRAB_RADIUS),
                self.get_pattern_status(self.right.selected, self.right.highlighted),
            ),
            (
                modifiers_path(self.get_bottom_modifier(), 1., ShapeRectangle::GRAB_RADIUS),
                self.get_pattern_status(self.bottom.selected, self.bottom.highlighted),
            ),
            (
                modifiers_path(self.get_left_modifier(), 1., ShapeRectangle::GRAB_RADIUS),
                self.get_pattern_status(self.left.selected, self.left.highlighted),
            ),
            (
                center_path(
                    (self.tl.pos + self.br.pos) / 2.,
                    1.,
                    ShapeRectangle::GRAB_RADIUS,
                ),
                self.get_pattern_status(self.selected, self.highlighted),
            ),
        ]
    }
    fn get_dimensions_paths_and_patterns(
        &self,
        _: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        let mut res = vec![];

        let dim = Dimension::new(
            DimKind::Horizontal,
            self.tl.pos,
            self.get_tr_modifier(),
            self.get_width(),
        )
        .get_path_and_pattern();
        res.push(dim);

        let dim = Dimension::new(
            DimKind::Vertical,
            self.get_bl_modifier(),
            self.tl.pos,
            self.get_height(),
        )
        .get_path_and_pattern();
        res.push(dim);

        res
    }
    fn get_paths_and_patterns(&self, _das: &Size, _: (Rect, f64, Vec2)) -> Vec<(BezPath, Pattern)> {
        let pattern = match (self.selected, self.highlighted) {
            (false, false) => Pattern::BasicNormal,
            (false, true) => Pattern::BasicHighlighted,
            (true, false) => Pattern::BasicSelected,
            (true, true) => Pattern::BasicSelected,
        };
        vec![(self.to_path(Self::TOLERANCE), pattern)]
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
