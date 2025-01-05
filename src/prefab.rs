// #![cfg(not(test))]
// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }

use kurbo::{BezPath, Circle, PathEl, Shape, Vec2};

use crate::math::MyError;

pub fn line_helper(pos1: &Vec2, pos2: &Vec2) -> Result<BezPath, MyError> {
    const EXTENSION: f64 = 100.;
    // Extend the line beyond the handles
    let ln = (*pos1 - *pos2).hypot();
    if ln == 0. {
        return Err(MyError::Impossible);
    };
    let ext_x = (pos2.x - pos1.x) / ln;
    let ext_y = (pos2.y - pos1.y) / ln;
    let pos1_ext = Vec2::new(pos1.x - EXTENSION * ext_x, pos1.y - EXTENSION * ext_y);
    let pos2_ext = Vec2::new(pos2.x + EXTENSION * ext_x, pos2.y + EXTENSION * ext_y);
    Ok(BezPath::from_vec(vec![
        PathEl::MoveTo(pos1_ext.to_point()),
        PathEl::LineTo(pos2_ext.to_point()),
    ]))
}

pub fn arrow_right(pos: &Vec2, size: f64) -> BezPath {
    use PathEl::*;
    let v: Vec<PathEl> = vec![
        MoveTo(pos.to_point()),
        LineTo(pos.to_point() + (size, 0.)),
        LineTo(pos.to_point() + (size + -5., -5.)),
        LineTo(pos.to_point() + (size + -5., 5.)),
        LineTo(pos.to_point() + (size, 0.)),
    ];
    BezPath::from_vec(v)
}

pub fn arrow_down(pos: &Vec2, size: f64) -> BezPath {
    use PathEl::*;
    let v: Vec<PathEl> = vec![
        MoveTo(pos.to_point()),
        LineTo(pos.to_point() + (0., size)),
        LineTo(pos.to_point() + (-5., size + -5.)),
        LineTo(pos.to_point() + (5., size + -5.)),
        LineTo(pos.to_point() + (0., size)),
    ];
    BezPath::from_vec(v)
}

pub fn center_path(pos: Vec2, _scale: f64, size: f64) -> BezPath {
    use PathEl::*;
    let v: Vec<PathEl> = vec![
        MoveTo(pos.to_point() - (0., size)),
        LineTo(pos.to_point() + (0., size)),
        MoveTo(pos.to_point() - (size, 0.)),
        LineTo(pos.to_point() + (size, 0.)),
    ];
    BezPath::from_vec(v)
}

pub fn modifiers_path(pos: Vec2, scale: f64, size: f64) -> BezPath {
    let tol = 0.01;
    let size = size;
    Circle::new(pos.to_point(), size / 2. / scale).to_path(tol)
}

pub fn handle_modify_path(pos: Vec2, scale: f64) -> BezPath {
    let tol = 0.01;
    let size = 2.;
    Circle::new(pos.to_point(), size / 2. / scale).to_path(tol)
}

pub fn cstr_hori(pos: &Vec2) -> BezPath {
    let size = 3.;
    use PathEl::*;
    let pos_offset = Vec2::new(pos.x - size / 2., pos.y - size / 2.);
    let v: Vec<PathEl> = vec![
        MoveTo(pos_offset.to_point()),
        LineTo(pos_offset.to_point() + (size, 0.)),
    ];
    BezPath::from_vec(v)
}

pub fn cstr_vert(pos: &Vec2) -> BezPath {
    let size = 3.;
    use PathEl::*;
    let pos_offset = Vec2::new(pos.x - size / 2., pos.y - size / 2.);
    let v: Vec<PathEl> = vec![
        MoveTo(pos_offset.to_point()),
        LineTo(pos_offset.to_point() + (0., size)),
    ];
    BezPath::from_vec(v)
}

pub fn line_45_scale_invariant(pos: &Vec2) -> BezPath {
    let size = 10.;
    use PathEl::*;
    let pos_offset = Vec2::new(pos.x - size, pos.y - size / 2.);
    let v: Vec<PathEl> = vec![
        MoveTo(pos_offset.to_point()),
        LineTo(pos_offset.to_point() + (size / 1.414, -size / 1.414)),
    ];
    BezPath::from_vec(v)
}

pub fn line_135_scale_invariant(pos: &Vec2) -> BezPath {
    let size = 10.;
    use PathEl::*;
    let pos_offset = Vec2::new(pos.x + size / 2., pos.y - size);
    let v: Vec<PathEl> = vec![
        MoveTo(pos_offset.to_point()),
        LineTo(pos_offset.to_point() + (size / 1.414, size / 1.414)),
    ];
    BezPath::from_vec(v)
}
