#![allow(dead_code)]
// A macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

use crate::closed_shapes::ClosedShapeId;
use approx::*;
use kurbo::{BezPath, Line, ParamCurveNearest, PathEl, Rect, RoundedRectRadii, Vec2};
use std::f64::consts::PI;
use std::{
    error::Error,
    f64::consts::*,
    fmt::{self},
};

#[derive(Debug)]
pub enum MyError {
    NoShapeSelected,
    NoClosedShapeForCShid(ClosedShapeId),
    Inconsistent,
    Impossible,
    ShapesFull,
    ShapesEmpty,
}
impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use MyError::*;
        match self {
            NoShapeSelected => write!(f, "No shape selected"),
            NoClosedShapeForCShid(shid) => {
                write!(f, "No closed shape associated with the shape id {}", shid)
            }
            Inconsistent => write!(f, "Inconsistant data structure"),
            Impossible => write!(f, "Impossible"),
            ShapesFull => write!(f, "Shapes full"),
            ShapesEmpty => write!(f, "Shapes empty"),
        }
    }
}
impl Error for MyError {}

pub fn is_aligned_vert(pt1: &Vec2, pt2: &Vec2) -> bool {
    // I can do this because of snaping
    (pt1.x - pt2.x).abs() == 0.
}
pub fn helper_vertical(pt1: &Vec2, pt2: &Vec2, full: bool) -> BezPath {
    use PathEl::*;
    let mut v: Vec<PathEl> = vec![];
    let y1 = 2. * pt1.y - pt2.y;
    let y2 = 2. * pt2.y - pt1.y;

    v.push(MoveTo(pt1.to_point()));
    if full {
        v.push(LineTo(Vec2::new(pt1.x, y1).to_point()));
        v.push(LineTo(pt2.to_point()));
        v.push(LineTo(Vec2::new(pt1.x, y2).to_point()));
    } else {
        v.push(LineTo(Vec2::new(pt1.x, y1).to_point()));
        v.push(MoveTo(pt2.to_point()));
        v.push(LineTo(Vec2::new(pt1.x, y2).to_point()));
    }
    BezPath::from_vec(v)
}

pub fn is_aligned_hori(pt1: &Vec2, pt2: &Vec2) -> bool {
    // I can do this because of snaping
    (pt1.y - pt2.y).abs() == 0.
}
pub fn helper_horizontal(pt1: &Vec2, pt2: &Vec2, full: bool) -> BezPath {
    use PathEl::*;
    let mut v: Vec<PathEl> = vec![];
    let x1 = 2. * pt1.x - pt2.x;
    let x2 = 2. * pt2.x - pt1.x;

    v.push(MoveTo(pt1.to_point()));
    if full {
        v.push(LineTo(Vec2::new(x1, pt1.y).to_point()));
        v.push(LineTo(pt2.to_point()));
        v.push(LineTo(Vec2::new(x2, pt1.y).to_point()));
    } else {
        v.push(LineTo(Vec2::new(x1, pt1.y).to_point()));
        v.push(MoveTo(pt2.to_point()));
        v.push(LineTo(Vec2::new(x2, pt1.y).to_point()));
    }
    BezPath::from_vec(v)
}

pub fn _is_aligned_45_or_135(pt1: &Vec2, pt2: &Vec2) -> bool {
    let dy = pt2.y - pt1.y;
    let dx = pt2.x - pt1.x;
    if dx != 0. {
        let m = (dy / dx).abs();
        // Equality test works because of snapping
        m == 1.
    } else {
        false
    }
}
pub fn _helper_45_135(pt1: &Vec2, pt2: &Vec2, full: bool) -> BezPath {
    use PathEl::*;
    let mut v: Vec<PathEl> = vec![];
    let x1 = 2. * pt1.x - pt2.x;
    let y1 = 2. * pt1.y - pt2.y;
    let x2 = 2. * pt2.x - pt1.x;
    let y2 = 2. * pt2.y - pt1.y;

    v.push(MoveTo(pt1.to_point()));
    if full {
        v.push(LineTo(Vec2::new(x1, y1).to_point()));
        v.push(LineTo(pt2.to_point()));
        v.push(LineTo(Vec2::new(x2, y2).to_point()));
    } else {
        v.push(LineTo(Vec2::new(x1, y1).to_point()));
        v.push(MoveTo(pt2.to_point()));
        v.push(LineTo(Vec2::new(x2, y2).to_point()));
    }

    BezPath::from_vec(v)
}

fn _is_between(pt: &Vec2, pt1: &Vec2, pt2: &Vec2) -> bool {
    let dot_product = (pt.x - pt1.x) * (pt2.x - pt1.x) + (pt.y - pt1.y) * (pt2.y - pt1.y);
    if dot_product < 0. {
        return false;
    }
    let length2 = (pt2.x - pt1.x).powf(2.) + (pt2.y - pt1.y).powf(2.);
    if dot_product > length2 {
        return false;
    }
    return true;
}
// pub fn is_Vec2_on_segment(pos1: &Vec2, pos2: &Vec2, pos: &Vec2, precision: f64) -> bool {
//     let denominator = ((pos2.y - pos1.y).powf(2.) + (pos2.x - pos1.x).powf(2.)).sqrt();
//     if denominator == 0. {
//         return is_Vec2_on_Vec2(pos, &pos1, precision);
//     }
//     let numerator = ((pos2.y - pos1.y) * pos.x - (pos2.x - pos1.x) * pos.y
//         + pos2.x * pos1.y
//         - pos2.y * pos1.x)
//         .abs();
//     if numerator / denominator > precision {
//         return false;
//     }
//     is_between(pos, &pos1, &pos2)
// }

pub fn is_box_inside(box_outer: &Rect, box_inner: &Rect) -> bool {
    box_outer.contains_rect(*box_inner)
}
pub fn reorder_corners(bb: &mut [Vec2; 2]) {
    let pt1 = bb[0];
    let pt2 = bb[1];
    if pt1.x < pt2.x {
        if pt1.y < pt2.y {
            let bl = Vec2 { x: pt1.x, y: pt1.y };
            let tr = Vec2 { x: pt2.x, y: pt2.y };
            bb[0] = bl;
            bb[1] = tr;
        } else {
            let bl = Vec2 { x: pt1.x, y: pt2.y };
            let tr = Vec2 { x: pt2.x, y: pt1.y };
            bb[0] = bl;
            bb[1] = tr;
        }
    } else {
        if pt1.y < pt2.y {
            let bl = Vec2 { x: pt2.x, y: pt1.y };
            let tr = Vec2 { x: pt1.x, y: pt2.y };
            bb[0] = bl;
            bb[1] = tr;
        } else {
            let bl = Vec2 { x: pt2.x, y: pt2.y };
            let tr = Vec2 { x: pt1.x, y: pt1.y };
            bb[0] = bl;
            bb[1] = tr;
        }
    }
}

pub fn _snap_to_positive_value(value: f64, snap_value: f64) -> f64 {
    let value = (value / snap_value).round() * snap_value;
    if value == 0. {
        snap_value
    } else {
        if value < 0. {
            -value
        } else {
            value
        }
    }
}

pub fn is_near_position(pos: Vec2, other_pos: Vec2, grab_handle_precision: f64) -> bool {
    (pos - other_pos).hypot() < grab_handle_precision / 2.
}

// pub fn is_line_on_helper(pt1: &Vec2, pt2: &Vec2, precision: f64) -> Option<Angled> {
//     let angle = get_line_angle(pt1, pt2);
//     const VALS: [f64; 9] = [
//         0.0,
//         PI,
//         FRAC_PI_2,
//         -FRAC_PI_2,
//         FRAC_PI_4,
//         -FRAC_PI_4,
//         3. * FRAC_PI_4,
//         -3. * FRAC_PI_4,
//         -PI,
//     ];
//     for (i, val) in VALS.iter().enumerate() {
//         match i {
//             0 | 1 | 8 => {
//                 if val.abs_diff_eq(&angle, precision) {
//                     return Some(Angled::Horizontal);
//                 }
//             }
//             2 | 3 => {
//                 if val.abs_diff_eq(&angle, precision) {
//                     return Some(Angled::Vertical);
//                 }
//             }
//             4 | 7 => {
//                 if val.abs_diff_eq(&angle, precision) {
//                     return Some(Angled::Inclined135);
//                 }
//             }
//             5 | 6 => {
//                 if val.abs_diff_eq(&angle, precision) {
//                     return Some(Angled::Inclined45);
//                 }
//             }
//             _ => unreachable!(),
//         }
//     }
//     None
// }

// Project v1(pt2-pos) vector to the perpendicular of vector v2(pt1-pt2)
pub fn get_unit_vector_perpendicular(pt1: &Vec2, pt2: &Vec2, pos: &Vec2) -> Option<Vec2> {
    let v1 = *pos - *pt2;
    let v2 = *pt2 - *pt1;
    if v2.hypot2() == 0. {
        return None;
    }
    let pv2 = Vec2::new(-v2.y, v2.x).normalize();
    let dot_product = v1.dot(pv2) * pv2;

    Some(dot_product)
}

pub fn get_dist_to_line(pt1: Vec2, pt2: Vec2, pos: Vec2) -> f64 {
    let v1 = pt2 - pt1; // Vector from pt1 to pt2
    let v2 = pos - pt1; // Vector from pt1 to pos

    // Compute the cross product magnitude between v1 and v2
    let cross = v1.x * v2.y - v1.y * v2.x;

    // Compute the length of v1 (the base of the triangle)
    let length_v1 = v1.hypot();

    if length_v1 == 0.0 {
        // If pt1 and pt2 are the same point, return the distance to the point
        return v2.hypot();
    }

    // Distance is the height of the triangle formed
    cross.abs() / length_v1
}

// intersection of the perpendicular to the line
// segment defined by pt1-pt2 passing through pt1,
// and the line parallel to pt1-pt2 passing through pos
pub fn get_intersection(pt1: Vec2, pt2: Vec2, pos: Vec2) -> Vec2 {
    let v1 = pt2 - pt1; // Vector representing pt1-pt2
    if v1.hypot() == 0.0 {
        // return None; // Handle edge case where pt1 and pt2 are the same
        return pt1;
    }

    // Normalize the direction vector of pt1-pt2
    let direction = v1.normalize();

    // Perpendicular vector to the direction of pt1-pt2
    let perpendicular = Vec2::new(-direction.y, direction.x);

    // Parametric equations for the lines:
    // Line 1 (perpendicular through pt1): L1(t) = pt1 + t * perpendicular
    // Line 2 (parallel through pos): L2(s) = pos + s * direction

    // Find t and s where L1(t) = L2(s)
    let determinant = perpendicular.x * direction.y - perpendicular.y * direction.x;
    if determinant.abs() < 1e-10 {
        // return None; // The lines are parallel and do not intersect
        return pt1;
    }

    let diff = pos - pt1;
    let t = (diff.x * direction.y - diff.y * direction.x) / determinant;

    // Intersection point
    pt1 + t * perpendicular
}
pub fn symmetric_point(pt1: Vec2, pt2: Vec2, pos: Vec2) -> Vec2 {
    let v1 = pt2 - pt1; // Vector representing pt1-pt2
    if v1.hypot() == 0.0 {
        // return None; // Handle edge case where pt1 and pt2 are the same
        return pt1;
    }

    // Direction vector normalized
    let direction = v1.normalize();

    // Vector from pt1 to pos
    let to_pos = pos - pt1;

    // Project `to_pos` onto the line direction
    let projection_length = to_pos.dot(direction);
    let projection = pt1 + projection_length * direction;

    // Calculate symmetric point
    let symmetric = 2.0 * projection - pos;

    symmetric
}

pub fn projection_to_perpendicular(pt1: Vec2, pt2: Vec2, pos: Vec2) -> Vec2 {
    let v1 = pt2 - pt1; // Vector from pt1 to pt2
    if v1.hypot() == 0.0 {
        // return None; // Handle edge case where pt1 and pt2 are the same
        return pt1;
    }

    // Midpoint of pt1 and pt2
    let midpoint = (pt1 + pt2) / 2.0;

    // Perpendicular vector to v1
    let perpendicular = Vec2::new(-v1.y, v1.x).normalize();

    // Vector from midpoint to pos
    let to_pos = pos - midpoint;

    // Projection of to_pos onto the perpendicular vector
    let projection_length = to_pos.dot(perpendicular);
    let projection = midpoint + projection_length * perpendicular;

    projection
}
pub fn point_at_distance(pt1: Vec2, pt2: Vec2, distance: f64) -> Vec2 {
    let v1 = pt2 - pt1; // Vector from pt1 to pt2
    if v1.hypot() == 0.0 {
        // return None; // Handle edge case where pt1 and pt2 are the same
        return pt1;
    }

    // Midpoint of pt1 and pt2
    let midpoint = (pt1 + pt2) * 0.5;

    // Perpendicular vector to v1, normalized
    let perpendicular = Vec2::new(-v1.y, v1.x).normalize();

    // Calculate the target point
    let result = midpoint + distance * perpendicular;

    result
}

pub fn angle_between(pt1: Vec2, pt2: Vec2) -> f64 {
    let vector = pt2 - pt1; // Vector from pt1 to pt2
    let angle = vector.y.atan2(vector.x); // atan2 gives the angle in radians
    angle
}

pub fn magnet_to_grid(pt: &Vec2) -> Vec2 {
    pt.round()
}

pub fn magnet_to_helpers(pt: &Vec2, other_pt: &Vec2, magnet_angle: f64) -> Vec2 {
    let angle = get_line_angle(pt, other_pt);
    let dpos = Vec2::new(pt.x - other_pt.x, pt.y - other_pt.y);

    match angle {
        a if a.abs_diff_eq(&0.0, magnet_angle) || a.abs_diff_eq(&PI, magnet_angle) => {
            log!("magnet O or pi");
            Vec2::new(pt.x, other_pt.y)
        }
        a if a.abs_diff_eq(&FRAC_PI_2, magnet_angle)
            || a.abs_diff_eq(&-FRAC_PI_2, magnet_angle) =>
        {
            log!("magnet +- pi/2");
            Vec2::new(other_pt.x, pt.y)
        }
        a if a.abs_diff_eq(&FRAC_PI_4, magnet_angle)
            || a.abs_diff_eq(&(-3. * FRAC_PI_4), magnet_angle) =>
        {
            log!("magnet pi/4 or -3pi/4");
            Vec2::new(other_pt.x + dpos.x, other_pt.y + dpos.x)
        }
        a if a.abs_diff_eq(&-FRAC_PI_4, magnet_angle)
            || a.abs_diff_eq(&(3. * FRAC_PI_4), magnet_angle) =>
        {
            log!("magnet -pi/4 or 3pi/4");
            Vec2::new(other_pt.x + dpos.x, other_pt.y - dpos.x)
        }
        _ => *pt,
    }
}

pub fn magnet_double_to_helpers(
    pos: &Vec2,
    other1_pt: &Vec2,
    other2_pt: &Vec2,
    magnet_angle: f64,
) -> Vec2 {
    let pos1 = magnet_to_helpers(pos, other1_pt, magnet_angle);
    magnet_to_helpers(&pos1, other2_pt, magnet_angle)
}

pub fn _pos_to_polar(pos1: &Vec2, pos2: &Vec2) -> (Vec2, f64) {
    let (x1, y1) = (pos1.x, pos1.y);
    let (x2, y2) = (pos2.x, pos2.y);

    let angle = (y2 - y1).atan2(x2 - x1);
    let rho = if x2 != x1 {
        let b = (y1 * x2 - x1 * y2) / (x2 - x1);
        let m = (y2 - y1) / (x2 - x1);
        Vec2::new(-m / (m * m + 1.) * b, b / (m * m + 1.))
    } else {
        Vec2::new(0., 0.)
    };

    (rho, angle)
}

// // Quad Bezier curve spliting (1 pt)
// #[allow(dead_code)]
// pub fn split_quad_bezier(
//     pt: &WVec2,
//     shape: &SimpleShape,
// ) -> Option<(SimpleShape, SimpleShape)> {
//     if let SimpleShape::QuadBezier(start, ctrl, end) = shape {
//         if let Some(t) = find_t_for_Vec2_on_quad_bezier(pt, &start.1, &ctrl.1, &end.1) {
//             let ctrl1 = start.1.lerp(&ctrl.1, t);
//             let ctrl2 = ctrl.1.lerp(&end.1, t);
//             let split = ctrl1.lerp(&ctrl2, t);
//             Some((
//                 SimpleShape::QuadBezier(
//                     (Handle::Start, start.1.clone()),
//                     (Handle::Ctrl, ctrl1),
//                     (Handle::End, split),
//                 ),
//                 SimpleShape::QuadBezier(
//                     (Handle::Start, split),
//                     (Handle::Ctrl, ctrl2),
//                     (Handle::End, end.1.clone()),
//                 ),
//             ))
//         } else {
//             None
//         }
//     } else {
//         None
//     }
// }
// // Cubic Bezier curve spliting (1 pt)
// #[allow(dead_code)]
// pub fn split_cubic_bezier(
//     pt: &WVec2,
//     shape: SimpleShape,
// ) -> Option<(SimpleShape, SimpleShape)> {
//     if let SimpleShape::CubicBezier(start, ctrl1, ctrl2, end) = shape {
//         if let Some(t) = find_t_for_Vec2_on_cubic_bezier(pt, &start.1, &ctrl1.1, &ctrl2.1, &end.1)
//         {
//             let p0_prime = start.1.lerp(&ctrl1.1, t);
//             let p1_prime = ctrl1.1.lerp(&ctrl2.1, t);
//             let p2_prime = ctrl2.1.lerp(&end.1, t);
//             let q0 = p0_prime.lerp(&p1_prime, t);
//             let q1 = p1_prime.lerp(&p2_prime, t);
//             let r = q0.lerp(&q1, t);
//             Some((
//                 SimpleShape::CubicBezier(
//                     (Handle::Start, start.1.clone()),
//                     (Handle::Ctrl, p0_prime),
//                     (Handle::End, q0),
//                     (Handle::End, r.clone()),
//                 ),
//                 SimpleShape::CubicBezier(
//                     (Handle::Start, r),
//                     (Handle::Ctrl, q1),
//                     (Handle::End, p2_prime),
//                     (Handle::End, end.1.clone()),
//                 ),
//             ))
//         } else {
//             None
//         }
//     } else {
//         None
//     }
// }
// // Rectangle spliting (2 pts)
// #[allow(dead_code)]
// pub fn split_rectangle(
//     pt1: &WVec2,
//     pt2: &WVec2,
//     shape: SimpleShape,
// ) -> Option<(SimpleShape, SimpleShape)> {
//     if pt1.dist(pt2) > EPSILON {
//         if let SimpleShape::Rectangle(bl, tl, tr, br) = shape {
//             let lines = vec![
//                 SimpleShape::Line((Handle::Start, bl), (Handle::End, tl)),
//                 SimpleShape::Line((Handle::Start, tl), (Handle::End, tr)),
//                 SimpleShape::Line((Handle::Start, tr), (Handle::End, br)),
//                 SimpleShape::Line((Handle::Start, br), (Handle::End, bl)),
//             ];
//             let mut oidx1 = None;
//             let mut oidx2 = None;
//             for (idx, line) in lines.iter().enumerate() {
//                 if let Some(v) = split_line(pt1, &line) {
//                     oidx1 = Some(idx);
//                     break;
//                 }
//             }
//             for (idx, line) in lines.iter().enumerate() {
//                 if let Some(v) = split_line(pt2, &line) {
//                     oidx2 = Some(idx);
//                     break;
//                 }
//             }
//             if let Some(idx1) = oidx1 {
//                 if let Some(idx2) = oidx2 {
//                     // TBD
//                     None
//                 } else {
//                     None
//                 }
//             } else {
//                 None
//             }
//             //
//         } else {
//             None
//         }
//     } else {
//         None
//     }
// }
// // Ellipse curve splitting (2 pts)
// #[allow(dead_code)]
// pub fn split_ellipse(
//     pt1: &WVec2,
//     pt2: &WVec2,
//     shape: SimpleShape,
// ) -> Option<(SimpleShape, SimpleShape)> {
//     if let SimpleShape::Ellipse(
//         center,
//         radius,
//         h_start_angle,
//         h_end_angle,
//         (rotation, start_angle, end_angle),
//     ) = shape
//     {
//         if pt1.dist(pt2) > EPSILON {
//             // Getting the angles for pt1 and pt2
//             let angle_pt1 = get_angle_from_Vec2(pt1, &center.1, rotation);
//             let angle_pt2 = get_angle_from_Vec2(pt2, &center.1, rotation);
//             // Ensuring angle_pt1 is smaller than angle_pt2
//             let (min_angle, max_angle) = if angle_pt1 < angle_pt2 {
//                 (angle_pt1, angle_pt2)
//             } else {
//                 (angle_pt2, angle_pt1)
//             };
//             let h_start_angle = get_Vec2_from_angle(&center.1, &radius.1, rotation, -start_angle);
//             let h_min_angle = get_Vec2_from_angle(&center.1, &radius.1, rotation, -min_angle);
//             let h_max_angle = get_Vec2_from_angle(&center.1, &radius.1, rotation, -max_angle);
//             let h_end_angle = get_Vec2_from_angle(&center.1, &radius.1, rotation, -end_angle);
//             Some((
//                 SimpleShape::Ellipse(
//                     (Handle::Center, center.1.clone()),
//                     (Handle::End, center.1.clone() + radius.1.clone()),
//                     (Handle::StartAngle, h_start_angle.addxy(center.1.x, 0.)),
//                     (Handle::EndAngle, h_min_angle.addxy(center.1.x, 0.)),
//                     (rotation, start_angle, min_angle),
//                 ),
//                 SimpleShape::Ellipse(
//                     (Handle::Center, center.1.clone()),
//                     (Handle::End, center.1.clone() + radius.1.clone()),
//                     (Handle::StartAngle, h_max_angle.addxy(center.1.x, 0.)),
//                     (Handle::EndAngle, h_end_angle.addxy(center.1.x, 0.)),
//                     (rotation, max_angle, end_angle),
//                 ),
//             ))
//         } else {
//             None
//         }
//     } else {
//         None
//     }
// }
// pub fn _get_Vec2_on_cubic_bezier(
//     t: f64,
//     start: &Vec2,
//     ctrl1: &Vec2,
//     ctrl2: &Vec2,
//     end: &Vec2,
// ) -> Vec2 {
// let u = 1.0 - t;
// let tt = t * t;
// let uu = u * u;
// let uuu = uu * u;
// let ttt = tt * t;
// let mut result = *start * uuu; // (1-t)^3 * start
// result += *ctrl1 * 3.0 * uu * t; // 3(1-t)^2 * t * ctrl1
// result += *ctrl2 * 3.0 * u * tt; // 3(1-t) * t^2 * ctrl2
// result += *end * ttt; // t^3 * end
// result
//TODO
//     Vec2::ZERO
// }

pub fn _get_vec2_from_angle(radius: &Vec2, angle: f64) -> Vec2 {
    let x = radius.x.abs() * angle.cos();
    let y = radius.y.abs() * angle.sin();
    Vec2 { x: x, y: y }
}

#[inline]
pub fn get_line_angle(pt2: &Vec2, pt1: &Vec2) -> f64 {
    let pt = *pt2 - *pt1;
    pt.y.atan2(pt.x)
}

#[inline]
pub fn is_same_direction(pt2: &Vec2, pt1: &Vec2, angle: f64) -> bool {
    let angle1 = get_line_angle(pt2, pt1);
    (angle1.tan() - angle.tan()).abs() <= 0.0001
}

#[inline]
pub fn dotp(pt2: &Vec2, pt1: &Vec2) -> f64 {
    pt2.x * pt1.x + pt2.y * pt1.y
}

// #[inline]
// #[allow(dead_code)]
// fn find_t_for_Vec2_on_quad_bezier(p: &Vec2, start: &Vec2, ctrl: &Vec2, end: &Vec2) -> Option<f64> {
//     let mut t_min = 0.0;
//     let mut t_max = 1.0;
//     for _ in 0..MAX_ITERATIONS {
//         let t_mid = (t_min + t_max) / 2.0;
//         let mid_Vec2 = get_Vec2_on_quad_bezier(t_mid, start, ctrl, end);
//         let dist = mid_Vec2.dist(p);
//         if dist < EPSILON {
//             return Some(t_mid);
//         }
//         if get_Vec2_on_quad_bezier((t_min + t_mid) / 2.0, start, ctrl, end).dist(p) < dist {
//             t_max = t_mid;
//         } else {
//             t_min = t_mid;
//         }
//     }
//     None
// }
// #[allow(dead_code)]
// fn find_t_for_Vec2_on_cubic_bezier(
//     p: &Vec2,
//     start: &Vec2,
//     ctrl1: &Vec2,
//     ctrl2: &Vec2,
//     end: &Vec2,
// ) -> Option<f64> {
// let mut t_min = 0.0;
// let mut t_max = 1.0;
// for _ in 0..MAX_ITERATIONS {
//     let t_mid = (t_min + t_max) / 2.0;
//     let mid_Vec2 = get_Vec2_on_cubic_bezier(t_mid, start, ctrl1, ctrl2, end);
//     let dist = mid_Vec2.dist(p);
//     if dist < EPSILON {
//         return Some(t_mid);
//     }
//     if get_Vec2_on_cubic_bezier((t_min + t_mid) / 2.0, start, ctrl1, ctrl2, end).dist(p) < dist
//     {
//         t_max = t_mid;
//     } else {
//         t_min = t_mid;
//     }
// }
//TODO
//     None
// }

pub fn to_canvas(pt: &Vec2, scale: f64, offset: &Vec2) -> Vec2 {
    Vec2 {
        x: (pt.x * scale) + offset.x,
        y: (pt.y * scale) + offset.y,
    }
}
pub fn to_world(pt: &Vec2, scale: f64, offset: &Vec2) -> Vec2 {
    Vec2 {
        x: (pt.x - offset.x) / scale,
        y: (pt.y - offset.y) / scale,
    }
}
pub fn near_line(pos1: Vec2, pos2: Vec2, pos: Vec2, precision: f64) -> bool {
    Line::new(pos1.to_point(), pos2.to_point())
        .nearest(pos.to_point(), 0.)
        .distance_sq
        .sqrt()
        < precision / 4.
}

pub fn get_arc_vars(pos1: Vec2, pos2: Vec2, clockwise: bool) -> (Vec2, Vec2, f64, f64, f64) {
    let (center, radii, start_angle, sweep_angle) = match (pos1.x < pos2.x, pos1.y < pos2.y) {
        (false, false) => {
            if clockwise {
                (
                    Vec2::new(pos1.x, pos2.y),
                    Vec2::new(pos1.x - pos2.x, pos1.y - pos2.y),
                    PI,
                    -PI / 2.,
                )
            } else {
                (
                    Vec2::new(pos2.x, pos1.y),
                    Vec2::new(pos1.x - pos2.x, pos1.y - pos2.y),
                    0.,
                    -PI / 2.,
                )
            }
        }

        (false, true) => {
            if clockwise {
                (
                    Vec2::new(pos2.x, pos1.y),
                    Vec2::new(pos1.x - pos2.x, pos2.y - pos1.y),
                    0.,
                    PI / 2.,
                )
            } else {
                (
                    Vec2::new(pos1.x, pos2.y),
                    Vec2::new(pos1.x - pos2.x, pos2.y - pos1.y),
                    PI,
                    PI / 2.,
                )
            }
        }
        (true, false) => {
            if clockwise {
                (
                    Vec2::new(pos2.x, pos1.y),
                    Vec2::new(pos2.x - pos1.x, pos1.y - pos2.y),
                    3. * PI / 2.,
                    -PI / 2.,
                )
            } else {
                (
                    Vec2::new(pos1.x, pos2.y),
                    Vec2::new(pos2.x - pos1.x, pos1.y - pos2.y),
                    0.,
                    PI / 2.,
                )
            }
        }
        (true, true) => {
            if clockwise {
                (
                    Vec2::new(pos1.x, pos2.y),
                    Vec2::new(pos2.x - pos1.x, pos2.y - pos1.y),
                    0.,
                    -PI / 2.,
                )
            } else {
                (
                    Vec2::new(pos2.x, pos1.y),
                    Vec2::new(pos2.x - pos1.x, pos2.y - pos1.y),
                    PI,
                    -PI / 2.,
                )
            }
        }
    };
    (center, radii, start_angle, sweep_angle, 0.)
}

pub fn get_min_radius(radii: RoundedRectRadii) -> f64 {
    radii
        .top_left
        .min(radii.top_right)
        .min(radii.bottom_left)
        .min(radii.bottom_right)
}

pub fn get_max_radius(radii: RoundedRectRadii) -> f64 {
    radii
        .top_left
        .max(radii.top_right)
        .max(radii.bottom_left)
        .max(radii.bottom_right)
}
