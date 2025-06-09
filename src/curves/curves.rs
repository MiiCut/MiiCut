use crate::canvas::Pattern;
use crate::pools::HS;
use kurbo::{ArcAppendIter, CubicBezIter, Line, LinePathIter, PathEl, QuadBezIter, Vec2};

// pub struct ExtLinePathIter {
//     pub line: Line,
//     pub ix: usize,
// }
// impl Iterator for ExtLinePathIter {
//     type Item = PathEl;

//     fn next(&mut self) -> Option<PathEl> {
//         let pta = self.line.p1 - (self.line.p1 - self.line.p0) * 0.3;
//         self.ix += 1;
//         match self.ix {
//             1 => Some(PathEl::MoveTo(self.line.p0)),
//             2 => Some(PathEl::LineTo(pta)),
//             _ => None,
//         }
//     }
// }

// pub trait CurveControls {
//     const TOLERANCE: f64;
//     const GRAB: f64;

//     fn save_vars(&mut self);
//     fn restore_vars(&mut self);

//     fn get_state(&self, hs: HS) -> Option<Vec2>;
//     fn set_state(&mut self, hs: HS, state: bool);

//     fn get_pattern(&self, selected: bool, highlighted: bool) -> Pattern {
//         match (selected, highlighted) {
//             (false, false) => Pattern::BasicNormal,
//             (false, true) => Pattern::BasicHighlighted,
//             (true, false) => Pattern::BasicSelected,
//             (true, true) => Pattern::BasicSelected,
//         }
//     }
//     // fn path_elements(&self, start: Vec2, end: Vec2) -> PrimitiveKindIter;

//     // fn get_paths_and_patterns(
//     //     &self,
//     //     start: Vec2,
//     //     end: Vec2,
//     //     das: &Size,
//     //     parent_selected: bool,
//     //     parent_highlighted: bool,
//     // ) -> (BezPath, Pattern);
// }
