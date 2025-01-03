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
    positions::Position,
    prefab::magnet_path,
    shape_disc::HighLightOrSelect,
    shapes::{ShapeKind, Shapes},
};
use geo::{LineString, Polygon};
use kurbo::{
    BezPath, PathEl, Point, Rect, RoundedRect, RoundedRectPathIter, RoundedRectRadii, Shape, Vec2,
};
use std::fmt::Display;
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeRectRounded {
    tl: Position,
    tr: Position,
    br: Position,
    bl: Position,
    top: Position,
    right: Position,
    bottom: Position,
    left: Position,
    rad_tl_center: Position,
    rad_tr_center: Position,
    rad_br_center: Position,
    rad_bl_center: Position,
    center: Position,

    highlighted: bool,
    selected: bool,

    segs: BezPath,
    polygon: Polygon<f64>,
}
impl ShapeRectRounded {
    const MIN_SIZE: f64 = 20.;
    const RAD_MIN_SIZE: f64 = ShapeRectRounded::MIN_SIZE / 2.;

    fn update_polygon(&mut self) {
        log!("calc rect rounded polygon");
        self.segs = calc_segs(self.get_paths_patterns());
        self.polygon = calc_polygon(&self.segs);
    }
    fn get_radii(&self) -> RoundedRectRadii {
        RoundedRectRadii {
            top_left: (self.rad_tl_center.get_pos()).hypot() / 2_f64.sqrt(),
            top_right: (self.rad_tr_center.get_pos()).hypot() / 2_f64.sqrt(),
            bottom_right: (self.rad_br_center.get_pos()).hypot() / 2_f64.sqrt(),
            bottom_left: (self.rad_bl_center.get_pos()).hypot() / 2_f64.sqrt(),
        }
    }
    fn get_max_radius_size(&self) -> f64 {
        let tl = self.tl.get_pos();
        let tr = self.tr.get_pos();
        let br = self.br.get_pos();
        let bl = self.bl.get_pos();
        let center = self.center.get_pos();
        (center.x - tl.x)
            .abs()
            .min((center.y - tl.y).abs())
            .min((center.x - tr.x).abs())
            .min((center.y - tr.y).abs())
            .min((center.x - br.x).abs())
            .min((center.y - br.y).abs())
            .min((center.x - bl.x).abs())
            .min((center.y - bl.y).abs())
    }
    fn get_rectangle_rounded(&self) -> RoundedRect {
        let (tl_pos, br_pos) = (self.tl.get_pos(), self.br.get_pos());
        RoundedRect::new(tl_pos.x, tl_pos.y, br_pos.x, br_pos.y, self.get_radii())
    }
    fn update_edges_modifiers(&mut self) {
        self.top.set_pos(Vec2::new(
            (self.tl.get_pos().x + self.tr.get_pos().x) / 2.,
            self.tl.get_pos().y,
        ));
        self.left.set_pos(Vec2::new(
            self.tl.get_pos().x,
            (self.tl.get_pos().y + self.bl.get_pos().y) / 2.,
        ));
        self.bottom.set_pos(Vec2::new(
            (self.bl.get_pos().x + self.br.get_pos().x) / 2.,
            self.bl.get_pos().y,
        ));
        self.right.set_pos(Vec2::new(
            self.br.get_pos().x,
            (self.br.get_pos().y + self.tr.get_pos().y) / 2.,
        ));
    }
    fn update_outer_modifiers(&mut self) {
        self.tl
            .set_pos(Vec2::new(self.left.get_pos().x, self.top.get_pos().y));
        self.tr
            .set_pos(Vec2::new(self.right.get_pos().x, self.top.get_pos().y));
        self.br
            .set_pos(Vec2::new(self.right.get_pos().x, self.bottom.get_pos().y));
        self.bl
            .set_pos(Vec2::new(self.left.get_pos().x, self.bottom.get_pos().y));
    }
    fn update_radii(&mut self) {
        let max_radius_size = self.get_max_radius_size();

        let rad_tl_center_pos = Vec2::new(
            (self.rad_tl_center.get_pos().x)
                .min(max_radius_size)
                .max(ShapeRectRounded::RAD_MIN_SIZE),
            (self.rad_tl_center.get_pos().y)
                .min(max_radius_size)
                .max(ShapeRectRounded::RAD_MIN_SIZE),
        );
        self.rad_tl_center.set_pos(rad_tl_center_pos);

        let rad_tr_center_pos = Vec2::new(
            (self.rad_tr_center.get_pos().x)
                .max(-max_radius_size)
                .min(-ShapeRectRounded::RAD_MIN_SIZE),
            (self.rad_tr_center.get_pos().y)
                .min(max_radius_size)
                .max(ShapeRectRounded::RAD_MIN_SIZE),
        );
        self.rad_tr_center.set_pos(rad_tr_center_pos);

        let rad_br_center_pos = Vec2::new(
            (self.rad_br_center.get_pos().x)
                .max(-max_radius_size)
                .min(-ShapeRectRounded::RAD_MIN_SIZE),
            (self.rad_br_center.get_pos().y)
                .max(-max_radius_size)
                .min(-ShapeRectRounded::RAD_MIN_SIZE),
        );
        self.rad_br_center.set_pos(rad_br_center_pos);

        let rad_bl_center_pos = Vec2::new(
            (self.rad_bl_center.get_pos().x)
                .min(max_radius_size)
                .max(ShapeRectRounded::RAD_MIN_SIZE),
            (self.rad_bl_center.get_pos().y)
                .max(-max_radius_size)
                .min(-ShapeRectRounded::RAD_MIN_SIZE),
        );
        self.rad_bl_center.set_pos(rad_bl_center_pos);
    }
    fn update_center(&mut self) {
        self.center
            .set_pos((self.tl.get_pos() + self.br.get_pos()) / 2.);
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
impl Shapes for ShapeRectRounded {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 2.;

    fn new(pos1: Vec2, pos2: Vec2) -> ShapeKind {
        let tl = Position::new(pos1, true);
        let tr = Position::new(Vec2::new(pos2.x, pos1.y), true);
        let mut br = Position::new(pos2, true);
        br.select(true);
        let bl = Position::new(Vec2::new(pos1.x, pos2.y), true);
        let top = Position::new(Vec2::new((pos1.x + pos2.x) / 2., pos1.y), true);
        let right = Position::new(Vec2::new(pos2.x, (pos1.y + pos2.y) / 2.), true);
        let bottom = Position::new(Vec2::new((pos1.x + pos2.x) / 2., pos2.y), true);
        let left = Position::new(Vec2::new(pos1.x, (pos1.y + pos2.y) / 2.), true);
        let rad_tl_center = Position::new(
            Vec2::new(
                ShapeRectRounded::RAD_MIN_SIZE,
                ShapeRectRounded::RAD_MIN_SIZE,
            ),
            true,
        );
        let rad_tr_center = Position::new(
            Vec2::new(
                -ShapeRectRounded::RAD_MIN_SIZE,
                ShapeRectRounded::RAD_MIN_SIZE,
            ),
            true,
        );
        let rad_br_center = Position::new(
            Vec2::new(
                -ShapeRectRounded::RAD_MIN_SIZE,
                -ShapeRectRounded::RAD_MIN_SIZE,
            ),
            true,
        );
        let rad_bl_center = Position::new(
            Vec2::new(
                ShapeRectRounded::RAD_MIN_SIZE,
                -ShapeRectRounded::RAD_MIN_SIZE,
            ),
            true,
        );

        ShapeKind::RectangleRounded(ShapeRectRounded {
            tl,
            tr,
            br,
            bl,
            top,
            right,
            bottom,
            left,
            rad_tl_center,
            rad_tr_center,
            rad_br_center,
            rad_bl_center,
            center: Position::new((pos1 + pos2) / 2., true),
            highlighted: false,
            selected: false,
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }
    fn good_size(&self) -> bool {
        ((self.tl.get_pos().x - self.br.get_pos().x).abs() >= ShapeRectRounded::MIN_SIZE)
            && ((self.tl.get_pos().y - self.br.get_pos().y).abs() >= ShapeRectRounded::MIN_SIZE)
    }
    fn save_pos(&mut self) {
        self.tl.save_pos();
        self.tr.save_pos();
        self.br.save_pos();
        self.bl.save_pos();
        self.top.save_pos();
        self.right.save_pos();
        self.bottom.save_pos();
        self.left.save_pos();
        self.center.save_pos();

        self.rad_tl_center.save_pos();
        self.rad_tr_center.save_pos();
        self.rad_br_center.save_pos();
        self.rad_bl_center.save_pos();
    }
    fn toggle_prop(&mut self) {
        ()
    }
    fn get_paths_patterns(&self) -> Vec<(BezPath, Pattern)> {
        if self.good_size() {
            vec![(
                self.get_rectangle_rounded().to_path(Self::TOLERANCE),
                self.get_pattern(self.selected, self.highlighted),
            )]
        } else {
            vec![(BezPath::new(), Pattern::BasicNormal)]
        }
    }
    fn get_polygon(&self) -> Polygon<f64> {
        self.polygon.clone()
    }
    fn hors_from_pos(&mut self, pos: Vec2, hors: HighLightOrSelect) -> bool {
        match hors {
            HighLightOrSelect::Highlight => {
                self.highlighted = self.contains(pos.to_point());
                self.highlighted
            }
            HighLightOrSelect::Select => {
                self.selected = self.contains(pos.to_point());
                self.selected
            }
        }
    }
    fn hors_center_from_pos(&mut self, pos: Vec2, hors: HighLightOrSelect) -> bool {
        let center_hors = (pos - self.center.get_pos()).hypot() < Self::GRAB;
        match hors {
            HighLightOrSelect::Highlight => {
                self.center.highlight(center_hors);
                center_hors
            }
            HighLightOrSelect::Select => {
                self.center.select(center_hors);
                center_hors
            }
        }
    }
    fn hors_modifiers_from_pos(&mut self, pos: Vec2, hors: HighLightOrSelect) -> bool {
        let tl_hors = (pos - self.tl.get_pos()).hypot() < Self::GRAB;
        let tr_hors = (pos - self.tr.get_pos()).hypot() < Self::GRAB;
        let br_hors = (pos - self.br.get_pos()).hypot() < Self::GRAB;
        let bl_hors = (pos - self.bl.get_pos()).hypot() < Self::GRAB;
        let rad_tl_hors =
            (pos - (self.rad_tl_center.get_pos() + self.tl.get_pos())).hypot() < Self::GRAB;
        let rad_tr_hors =
            (pos - (self.rad_tr_center.get_pos() + self.tr.get_pos())).hypot() < Self::GRAB;
        let rad_br_hors =
            (pos - (self.rad_br_center.get_pos() + self.br.get_pos())).hypot() < Self::GRAB;
        let rad_bl_hors =
            (pos - (self.rad_bl_center.get_pos() + self.bl.get_pos())).hypot() < Self::GRAB;
        let top_hors = (pos - self.top.get_pos()).hypot() < Self::GRAB;
        let right_hors = (pos - self.right.get_pos()).hypot() < Self::GRAB;
        let bottom_hors = (pos - self.bottom.get_pos()).hypot() < Self::GRAB;
        let left_hors = (pos - self.left.get_pos()).hypot() < Self::GRAB;
        match hors {
            HighLightOrSelect::Highlight => {
                self.tl.highlight(tl_hors);
                self.tr.highlight(tr_hors);
                self.br.highlight(br_hors);
                self.bl.highlight(bl_hors);
                self.rad_tl_center.highlight(rad_tl_hors);
                self.rad_tr_center.highlight(rad_tr_hors);
                self.rad_br_center.highlight(rad_br_hors);
                self.rad_bl_center.highlight(rad_bl_hors);
                self.top.highlight(top_hors);
                self.right.highlight(right_hors);
                self.bottom.highlight(bottom_hors);
                self.left.highlight(left_hors);
                self.tl.is_highlighted()
                    || self.tr.is_highlighted()
                    || self.br.is_highlighted()
                    || self.bl.is_highlighted()
                    || self.rad_tl_center.is_highlighted()
                    || self.rad_tr_center.is_highlighted()
                    || self.rad_br_center.is_highlighted()
                    || self.rad_bl_center.is_highlighted()
                    || self.top.is_highlighted()
                    || self.right.is_highlighted()
                    || self.bottom.is_highlighted()
                    || self.left.is_highlighted()
            }
            HighLightOrSelect::Select => {
                self.tl.select(tl_hors);
                self.tr.select(tr_hors);
                self.br.select(br_hors);
                self.bl.select(bl_hors);
                self.rad_tl_center.select(rad_tl_hors);
                self.rad_tr_center.select(rad_tr_hors);
                self.rad_br_center.select(rad_br_hors);
                self.rad_bl_center.select(rad_bl_hors);
                self.top.select(top_hors);
                self.right.select(right_hors);
                self.bottom.select(bottom_hors);
                self.left.select(left_hors);
                self.tl.is_selected()
                    || self.tr.is_selected()
                    || self.br.is_selected()
                    || self.bl.is_selected()
                    || self.rad_tl_center.is_selected()
                    || self.rad_tr_center.is_selected()
                    || self.rad_br_center.is_selected()
                    || self.rad_bl_center.is_selected()
                    || self.top.is_selected()
                    || self.right.is_selected()
                    || self.bottom.is_selected()
                    || self.left.is_selected()
            }
        }
    }
    fn set_hors(&mut self, value: bool, hors: HighLightOrSelect) {
        match hors {
            HighLightOrSelect::Highlight => self.highlighted = value,
            HighLightOrSelect::Select => self.selected = value,
        }
    }
    fn set_hors_center(&mut self, value: bool, hors: HighLightOrSelect) {
        match hors {
            HighLightOrSelect::Highlight => self.center.highlight(value),
            HighLightOrSelect::Select => self.center.select(value),
        }
    }
    fn set_hors_modifiers(&mut self, value: bool, hors: HighLightOrSelect) {
        match hors {
            HighLightOrSelect::Highlight => {
                self.tl.highlight(value);
                self.tr.highlight(value);
                self.br.highlight(value);
                self.bl.highlight(value);
                self.top.highlight(value);
                self.right.highlight(value);
                self.bottom.highlight(value);
                self.left.highlight(value);
                self.rad_tl_center.highlight(value);
                self.rad_tr_center.highlight(value);
                self.rad_br_center.highlight(value);
                self.rad_bl_center.highlight(value);
            }
            HighLightOrSelect::Select => {
                self.tl.select(value);
                self.tr.select(value);
                self.br.select(value);
                self.bl.select(value);
                self.top.select(value);
                self.right.select(value);
                self.bottom.select(value);
                self.left.select(value);
                self.rad_tl_center.select(value);
                self.rad_tr_center.select(value);
                self.rad_br_center.select(value);
                self.rad_bl_center.select(value);
            }
        }
    }
    fn is_hors(&self, hors: HighLightOrSelect) -> bool {
        match hors {
            HighLightOrSelect::Highlight => self.highlighted,
            HighLightOrSelect::Select => self.selected,
        }
    }
    fn is_center_hors(&self, hors: HighLightOrSelect) -> bool {
        match hors {
            HighLightOrSelect::Highlight => self.center.is_highlighted(),
            HighLightOrSelect::Select => self.center.is_selected(),
        }
    }
    fn get_position(&self) -> Vec2 {
        self.center.get_pos()
    }
    fn move_position(&mut self, pos_init: Vec2, pos: Vec2, _shift_pressed: bool) {
        let tl_saved = self.tl.get_saved_pos();
        let tr_saved = self.tr.get_saved_pos();
        let br_saved = self.br.get_saved_pos();
        let bl_saved = self.bl.get_saved_pos();
        let top_saved = self.top.get_saved_pos();
        let right_saved = self.right.get_saved_pos();
        let bottom_saved = self.bottom.get_saved_pos();
        let left_saved = self.left.get_saved_pos();
        let center_saved = self.center.get_saved_pos();
        let rad_tl_center_saved = self.rad_tl_center.get_saved_pos();
        let rad_tr_center_saved = self.rad_tr_center.get_saved_pos();
        let rad_br_center_saved = self.rad_br_center.get_saved_pos();
        let rad_bl_center_saved = self.rad_bl_center.get_saved_pos();

        let tl_sel = self.tl.is_selected();
        let tr_sel = self.tr.is_selected();
        let br_sel = self.br.is_selected();
        let bl_sel = self.bl.is_selected();
        let top_sel = self.top.is_selected();
        let right_sel = self.right.is_selected();
        let bottom_sel = self.bottom.is_selected();
        let left_sel = self.left.is_selected();
        let rad_tl_sel = self.rad_tl_center.is_selected();
        let rad_tr_sel = self.rad_tr_center.is_selected();
        let rad_br_sel = self.rad_br_center.is_selected();
        let rad_bl_sel = self.rad_bl_center.is_selected();

        let dpos = pos - pos_init;
        const MIN_SIZE: f64 = ShapeRectRounded::MIN_SIZE;
        const RAD_MIN_SIZE: f64 = ShapeRectRounded::RAD_MIN_SIZE;

        if self.selected {
            self.center.set_pos(center_saved + dpos);
            self.tl.set_pos(tl_saved + dpos);
            self.tr.set_pos(tr_saved + dpos);
            self.br.set_pos(br_saved + dpos);
            self.bl.set_pos(bl_saved + dpos);
            self.top.set_pos(top_saved + dpos);
            self.right.set_pos(right_saved + dpos);
            self.bottom.set_pos(bottom_saved + dpos);
            self.left.set_pos(left_saved + dpos);
            self.update_polygon();
        } else {
            let modified = match (tl_sel, tr_sel, br_sel, bl_sel) {
                (true, false, false, false) => {
                    let mut tlpos = tl_saved + dpos;
                    tlpos.x = tlpos.x.min(tr_saved.x - MIN_SIZE);
                    tlpos.y = tlpos.y.min(bl_saved.y - MIN_SIZE);
                    self.tl.set_pos(tlpos);
                    self.bl.set_pos(Vec2::new(tlpos.x, bl_saved.y));
                    self.tr.set_pos(Vec2::new(tr_saved.x, tlpos.y));
                    true
                }
                (false, true, false, false) => {
                    let mut trpos = tr_saved + dpos;
                    trpos.x = trpos.x.max(tl_saved.x + MIN_SIZE);
                    trpos.y = trpos.y.min(br_saved.y - MIN_SIZE);
                    self.tr.set_pos(trpos);
                    self.br.set_pos(Vec2::new(trpos.x, br_saved.y));
                    self.tl.set_pos(Vec2::new(tl_saved.x, trpos.y));
                    true
                }
                (false, false, true, false) => {
                    let mut brpos = br_saved + dpos;
                    brpos.x = brpos.x.max(bl_saved.x + MIN_SIZE);
                    brpos.y = brpos.y.max(tr_saved.y + MIN_SIZE);
                    self.br.set_pos(brpos);
                    self.tr.set_pos(Vec2::new(brpos.x, tr_saved.y));
                    self.bl.set_pos(Vec2::new(bl_saved.x, brpos.y));
                    true
                }
                (false, false, false, true) => {
                    let mut blpos = bl_saved + dpos;
                    blpos.x = blpos.x.min(br_saved.x - MIN_SIZE);
                    blpos.y = blpos.y.max(tl_saved.y + MIN_SIZE);
                    self.bl.set_pos(blpos);
                    self.tl.set_pos(Vec2::new(blpos.x, tl_saved.y));
                    self.br.set_pos(Vec2::new(br_saved.x, blpos.y));
                    true
                }
                _ => false,
            };

            if modified {
                self.update_edges_modifiers();
                self.update_radii();
                self.update_center();
                self.update_polygon();
                return;
            }

            let modified = match (top_sel, right_sel, bottom_sel, left_sel) {
                (true, false, false, false) => {
                    let mut top_pos = top_saved + Vec2::new(0., dpos.y);
                    top_pos.y = top_pos.y.min(bottom_saved.y - MIN_SIZE);
                    self.top.set_pos(top_pos);
                    self.left
                        .set_pos(Vec2::new(left_saved.x, (top_pos.y + bottom_saved.y) / 2.));
                    self.right
                        .set_pos(Vec2::new(right_saved.x, (top_pos.y + bottom_saved.y) / 2.));
                    true
                }
                (false, true, false, false) => {
                    let mut right_pos = right_saved + Vec2::new(dpos.x, 0.);
                    right_pos.x = right_pos.x.max(left_saved.x + MIN_SIZE);
                    self.right.set_pos(right_pos);
                    self.top
                        .set_pos(Vec2::new((left_saved.x + right_pos.x) / 2., top_saved.y));
                    self.bottom
                        .set_pos(Vec2::new((left_saved.x + right_pos.x) / 2., bottom_saved.y));
                    true
                }
                (false, false, true, false) => {
                    let mut bottom_pos = bottom_saved + Vec2::new(0., dpos.y);
                    bottom_pos.y = bottom_pos.y.max(top_saved.y + MIN_SIZE);
                    self.bottom.set_pos(bottom_pos);
                    self.left
                        .set_pos(Vec2::new(left_saved.x, (top_saved.y + bottom_pos.y) / 2.));
                    self.right
                        .set_pos(Vec2::new(right_saved.x, (top_saved.y + bottom_pos.y) / 2.));
                    true
                }
                (false, false, false, true) => {
                    let mut left_pos = left_saved + Vec2::new(dpos.x, 0.);
                    left_pos.x = left_pos.x.min(right_saved.x - MIN_SIZE);
                    self.left.set_pos(left_pos);
                    self.top
                        .set_pos(Vec2::new((left_pos.x + right_saved.x) / 2., top_saved.y));
                    self.bottom
                        .set_pos(Vec2::new((left_pos.x + right_saved.x) / 2., bottom_saved.y));
                    true
                }
                _ => false,
            };
            if modified {
                self.update_outer_modifiers();
                self.update_radii();
                self.update_center();
                self.update_polygon();
                return;
            }

            let max_radius_size = self.get_max_radius_size();
            let modified = match (rad_tl_sel, rad_tr_sel, rad_br_sel, rad_bl_sel) {
                (true, false, false, false) => {
                    let (mut proj, dir) = project_to_segment_with_direction(
                        self.tl.get_pos(),
                        self.center.get_pos(),
                        dpos,
                    );
                    proj /= 2_f64.sqrt();
                    let rad_tl_center_pos = Vec2::new(
                        (rad_tl_center_saved.x + proj * dir)
                            .max(RAD_MIN_SIZE)
                            .min(max_radius_size),
                        (rad_tl_center_saved.y + proj * dir)
                            .max(RAD_MIN_SIZE)
                            .min(max_radius_size),
                    );
                    self.rad_tl_center.set_pos(rad_tl_center_pos);
                    true
                }
                (false, true, false, false) => {
                    let (mut proj, dir) = project_to_segment_with_direction(
                        self.tr.get_pos(),
                        self.center.get_pos(),
                        dpos,
                    );
                    proj /= 2_f64.sqrt();
                    let rad_tr_center_pos = Vec2::new(
                        (rad_tr_center_saved.x - proj * dir)
                            .min(-RAD_MIN_SIZE)
                            .max(-max_radius_size),
                        (rad_tr_center_saved.y + proj * dir)
                            .max(RAD_MIN_SIZE)
                            .min(max_radius_size),
                    );
                    self.rad_tr_center.set_pos(rad_tr_center_pos);
                    true
                }
                (false, false, true, false) => {
                    let (mut proj, dir) = project_to_segment_with_direction(
                        self.br.get_pos(),
                        self.center.get_pos(),
                        dpos,
                    );
                    proj /= 2_f64.sqrt();
                    let rad_br_center_pos = Vec2::new(
                        (rad_br_center_saved.x - proj * dir)
                            .min(-RAD_MIN_SIZE)
                            .max(-max_radius_size),
                        (rad_br_center_saved.y - proj * dir)
                            .min(-RAD_MIN_SIZE)
                            .max(-max_radius_size),
                    );
                    self.rad_br_center.set_pos(rad_br_center_pos);
                    true
                }
                (false, false, false, true) => {
                    let (mut proj, dir) = project_to_segment_with_direction(
                        self.bl.get_pos(),
                        self.center.get_pos(),
                        dpos,
                    );
                    proj /= 2_f64.sqrt();
                    let rad_bl_center_pos = Vec2::new(
                        (rad_bl_center_saved.x + proj * dir)
                            .max(RAD_MIN_SIZE)
                            .min(max_radius_size),
                        (rad_bl_center_saved.y - proj * dir)
                            .min(-RAD_MIN_SIZE)
                            .max(-max_radius_size),
                    );
                    self.rad_bl_center.set_pos(rad_bl_center_pos);
                    true
                }
                _ => false,
            };
            if modified {
                self.update_polygon();
            }
        }
    }
    fn get_magnets_paths(&self) -> Vec<(BezPath, Pattern)> {
        let tl = self.tl.get_pos();
        let tr = self.tr.get_pos();
        let br = self.br.get_pos();
        let bl = self.bl.get_pos();
        let center = self.center.get_pos();
        let rad_tl_center = tl + self.rad_tl_center.get_pos();
        let rad_tr_center = tr + self.rad_tr_center.get_pos();
        let rad_br_center = br + self.rad_br_center.get_pos();
        let rad_bl_center = bl + self.rad_bl_center.get_pos();
        let top = self.top.get_pos();
        let right = self.right.get_pos();
        let bottom = self.bottom.get_pos();
        let left = self.left.get_pos();

        vec![
            (
                magnet_path(tl, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(self.tl.is_selected(), self.tl.is_highlighted()),
            ),
            (
                magnet_path(tr, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(self.tr.is_selected(), self.tr.is_highlighted()),
            ),
            (
                magnet_path(br, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(self.br.is_selected(), self.br.is_highlighted()),
            ),
            (
                magnet_path(bl, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(self.bl.is_selected(), self.bl.is_highlighted()),
            ),
            (
                magnet_path(top, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(self.top.is_selected(), self.top.is_highlighted()),
            ),
            (
                magnet_path(right, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(self.right.is_selected(), self.right.is_highlighted()),
            ),
            (
                magnet_path(bottom, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(self.bottom.is_selected(), self.bottom.is_highlighted()),
            ),
            (
                magnet_path(left, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(self.left.is_selected(), self.left.is_highlighted()),
            ),
            (
                magnet_path(center, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(self.center.is_selected(), self.center.is_highlighted()),
            ),
            (
                magnet_path(rad_tl_center, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(
                    self.rad_tl_center.is_selected(),
                    self.rad_tl_center.is_highlighted(),
                ),
            ),
            (
                magnet_path(rad_tr_center, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(
                    self.rad_tr_center.is_selected(),
                    self.rad_tr_center.is_highlighted(),
                ),
            ),
            (
                magnet_path(rad_br_center, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(
                    self.rad_br_center.is_selected(),
                    self.rad_br_center.is_highlighted(),
                ),
            ),
            (
                magnet_path(rad_bl_center, 1., ShapeRectRounded::GRAB),
                self.get_pattern_modifiers(
                    self.rad_bl_center.is_selected(),
                    self.rad_bl_center.is_highlighted(),
                ),
            ),
        ]
    }
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        let mut paths = vec![];
        let mut texts = vec![];
        let (path, text) =
            Dimension::new(DimKind::Horizontal, self.tl.get_pos(), self.tr.get_pos()).get_path();
        paths.push(path);
        texts.push(text);
        let (path, text) =
            Dimension::new(DimKind::Vertical, self.bl.get_pos(), self.tl.get_pos()).get_path();
        paths.push(path);
        texts.push(text);
        (paths, texts)
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
