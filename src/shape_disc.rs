// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }

use geo::{LineString, Polygon};
use kurbo::{BezPath, Circle, CirclePathIter, Point, Rect, Shape, Vec2};
use std::fmt::Display;

use crate::{
    canvas::{CanvasText, Pattern},
    dimensions::{DimKind, Dimension},
    math::*,
    positions::Position,
    prefab::magnet_path,
    shapes::{ShapeKind, Shapes},
};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum HighLightOrSelect {
    Highlight,
    Select,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeDisc {
    center: Position,
    radius_grab: Position,

    highlighted: bool,
    selected: bool,

    segs: BezPath,
    polygon: Polygon<f64>,
}
impl ShapeDisc {
    const MIN_SIZE: f64 = 2.;

    fn update_polygon(&mut self) {
        log!("calc disc polygon");
        self.segs = calc_segs(self.get_paths_patterns());
        self.polygon = calc_polygon(&self.segs);
    }
    fn get_circle(&self) -> Circle {
        Circle::new(
            self.center.get_pos().to_point(),
            self.radius_grab.get_pos().hypot() / 2_f64.sqrt(),
        )
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

impl Shapes for ShapeDisc {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 2.;

    fn new(pos1: Vec2, _pos2: Vec2) -> ShapeKind {
        let mut radius_grab =
            Position::new(Vec2::new(ShapeDisc::MIN_SIZE, ShapeDisc::MIN_SIZE), true);
        radius_grab.select(true);
        ShapeKind::Disc(ShapeDisc {
            center: Position::new(pos1, true),
            radius_grab,
            highlighted: false,
            selected: false,
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }
    fn good_size(&self) -> bool {
        self.radius_grab.get_pos().hypot() / 2_f64.sqrt() >= ShapeDisc::MIN_SIZE
    }
    fn save_pos(&mut self) {
        self.center.save_pos();
        self.radius_grab.save_pos();
    }
    fn toggle_prop(&mut self) {
        ()
    }
    fn get_paths_patterns(&self) -> Vec<(BezPath, Pattern)> {
        vec![(
            self.get_circle().to_path(Self::TOLERANCE),
            self.get_pattern(self.selected, self.highlighted),
        )]
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
        let center_hors = (pos - self.center.get_pos()).hypot() < ShapeDisc::GRAB;
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
        let radius_grab_hors =
            (pos - self.center.get_pos() - self.radius_grab.get_pos()).hypot() < ShapeDisc::GRAB;
        match hors {
            HighLightOrSelect::Highlight => {
                self.radius_grab.highlight(radius_grab_hors);
                self.radius_grab.is_highlighted()
            }
            HighLightOrSelect::Select => {
                self.radius_grab.select(radius_grab_hors);
                self.radius_grab.is_selected()
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
                self.radius_grab.highlight(value);
            }
            HighLightOrSelect::Select => {
                self.radius_grab.select(value);
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
        let c_saved = self.center.get_saved_pos();

        let dpos = pos - pos_init;

        if self.selected {
            self.center.set_pos(c_saved + dpos);
            self.update_polygon();
        } else {
            if self.radius_grab.is_selected() {
                let radius = pos - c_saved;
                if radius.hypot() < ShapeDisc::MIN_SIZE || radius.x < ShapeDisc::MIN_SIZE {
                    return;
                } else {
                    self.radius_grab.set_pos(Vec2::new(radius.x, radius.x));
                }
                self.update_polygon();
            }
        }
    }
    fn get_magnets_paths(&self) -> Vec<(BezPath, Pattern)> {
        vec![
            (
                magnet_path(self.center.get_pos(), 1., ShapeDisc::GRAB),
                self.get_pattern_modifiers(self.center.is_selected(), self.center.is_highlighted()),
            ),
            (
                magnet_path(
                    self.center.get_pos() + self.radius_grab.get_pos(),
                    1.,
                    ShapeDisc::GRAB,
                ),
                self.get_pattern_modifiers(
                    self.radius_grab.is_selected(),
                    self.radius_grab.is_highlighted(),
                ),
            ),
        ]
    }
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        let mut paths = vec![];
        let mut texts = vec![];
        let end = self.center.get_pos()
            + Vec2::new(
                self.radius_grab.get_pos().x * 0.8,
                -self.radius_grab.get_pos().y * 0.8,
            );
        let (path, text) = Dimension::new(DimKind::Radius, self.center.get_pos(), end).get_path();
        paths.push(path);
        texts.push(text);
        (paths, texts)
    }
}
