// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
use crate::shapes::shapes_pool::BSid;
use approx::*;
use geo::{LineString, Polygon};
use kurbo::{
    flatten, Arc, BezPath, Line, ParamCurveNearest, PathEl, Point, RoundedRectRadii, Size, Vec2,
};
use std::f64::consts::PI;
use std::{
    error::Error,
    f64::consts::*,
    fmt::{self},
};

pub const EPSILON: f64 = 1e-9;
// Snap the angle to horizontal or vertical
const THREAS_ANGLE: f64 = 2. / 180. * PI;

#[derive(Debug)]
pub enum MyError {
    NoShapeSelected,
    NoClosedShapeForCShid(BSid),
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

// Helper to calculate signed distance to a line
pub fn signed_distance(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    (p.x - a.x) * (b.y - a.y) - (p.y - a.y) * (b.x - a.x)
}

pub fn get_dist_to_segment(pt1: Vec2, pt2: Vec2, pos: Vec2) -> f64 {
    let v1 = pt2 - pt1; // Vector from pt1 to pt2
    let v2 = pos - pt1; // Vector from pt1 to pos
    let v3 = pos - pt2; // Vector from pt2 to pos

    // Compute the squared length of the segment
    let length_v1_sq = v1.hypot2();

    if length_v1_sq == 0.0 {
        // If pt1 and pt2 are the same point, return the distance to that point
        return v2.hypot();
    }

    // Project v2 onto v1 to find the perpendicular point on the line
    let t = v2.dot(v1) / length_v1_sq;

    if t < 0.0 {
        // Closest point is pt1
        v2.hypot()
    } else if t > 1.0 {
        // Closest point is pt2
        v3.hypot()
    } else {
        // Closest point is on the segment
        let projection = pt1 + t * v1;
        (pos - projection).hypot()
    }
}

// intersection of the perpendicular to the line
// segment defined by pt1-pt2 passing through pt1,
// and the line parallel to pt1-pt2 passing through pos
pub fn get_intersection(pt1: Vec2, pt2: Vec2, pos: Vec2) -> Vec2 {
    let v1 = pt2 - pt1; // Vector representing pt1-pt2
    if v1.hypot() == 0.0 {
        log!("WARNING: Degenerate case get_intersection()");
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
        log!("WARNING: Degenerate case symmetric_point()");
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
        log!("WARNING: Degenerate case projection_to_perpendicular()");
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
        log!("WARNING: Degenerate case point_at_distance()");
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

pub fn to_canvas(pt: Vec2, scale: f64, offset: Vec2) -> Vec2 {
    Vec2 {
        x: (pt.x * scale) + offset.x,
        y: (pt.y * scale) + offset.y,
    }
}
pub fn to_draw(pt: Vec2, scale: f64, offset: Vec2) -> Vec2 {
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

pub fn calculate_arc_points(arc: Arc) -> (Vec2, Vec2) {
    let center = arc.center; // Center of the arc
    let radii = arc.radii; // Radii of the ellipse
    let start_angle = arc.start_angle; // Start angle in radians
    let sweep_angle = arc.sweep_angle; // Sweep angle in radians
    let rotation = arc.x_rotation; // Rotation in radians

    // Start and end angles in radians
    let end_angle = start_angle + sweep_angle;

    // Function to compute an elliptical point without rotation
    let compute_point =
        |angle: f64| -> Vec2 { Vec2::new(radii.x * angle.cos(), radii.y * angle.sin()) };

    // Rotate a point around the origin by a given angle
    let rotate_point = |point: Vec2, angle: f64| -> Vec2 {
        Vec2::new(
            point.x * angle.cos() - point.y * angle.sin(),
            point.x * angle.sin() + point.y * angle.cos(),
        )
    };

    // Compute the unrotated start and end points
    let start_point_unrotated = compute_point(start_angle);
    let end_point_unrotated = compute_point(end_angle);

    // Rotate the points by the given rotation angle
    let start_point_rotated = rotate_point(start_point_unrotated, rotation);
    let end_point_rotated = rotate_point(end_point_unrotated, rotation);

    // Translate the points to the center
    let start_point = Vec2::new(
        center.x + start_point_rotated.x,
        center.y + start_point_rotated.y,
    );
    let end_point = Vec2::new(
        center.x + end_point_rotated.x,
        center.y + end_point_rotated.y,
    );

    (start_point, end_point)
}

pub fn is_point_near_path(path: &BezPath, point: Vec2, threshold: f64) -> bool {
    let threshold_squared = threshold * threshold;

    for el in path.elements() {
        match *el {
            kurbo::PathEl::MoveTo(p) | kurbo::PathEl::LineTo(p) => {
                if (p.x - point.x).powi(2) + (p.y - point.y).powi(2) <= threshold_squared {
                    return true;
                }
            }
            _ => {}
        }
    }

    false
}

/// Calculate the shortest distance from a point to a line segment.
pub fn distance_to_segment(start: Vec2, end: Vec2, pos: Vec2, grab_zone: f64) -> f64 {
    let a = start;
    let b = end;

    if (a - pos).hypot() < grab_zone {
        return f64::MAX;
    }
    if (b - pos).hypot() < grab_zone {
        return f64::MAX;
    }

    // Vector from a to b
    let ab = (b.x - a.x, b.y - a.y);

    // Vector from a to the point
    let ap = (pos.x - a.x, pos.y - a.y);

    // Project point onto the line segment, clamping t to [0, 1]
    let ab_length_squared = ab.0 * ab.0 + ab.1 * ab.1;
    let t = if ab_length_squared == 0.0 {
        0.0 // Degenerate segment
    } else {
        ((ap.0 * ab.0 + ap.1 * ab.1) / ab_length_squared).clamp(0.0, 1.0)
    };

    // Closest point on the segment
    let closest = Vec2::new(a.x + t * ab.0, a.y + t * ab.1);

    // Distance to the closest point
    (pos - closest).hypot()
}

pub fn compute_winding_number(path: &BezPath, point: Vec2) -> i32 {
    let mut winding_number = 0;
    let mut last_point = None;

    for element in path.elements() {
        match element {
            PathEl::MoveTo(p1) => {
                last_point = Some(p1);
            }
            PathEl::LineTo(p2) => {
                if let Some(p1) = last_point {
                    winding_number +=
                        segment_winding_contribution(p1.to_vec2(), p2.to_vec2(), point);
                }
                last_point = Some(p2);
            }
            PathEl::ClosePath => {
                if let Some(p1) = last_point {
                    if let Some(PathEl::MoveTo(p_start)) = path.elements().first() {
                        winding_number +=
                            segment_winding_contribution(p1.to_vec2(), p_start.to_vec2(), point);
                    }
                }
            }
            _ => {}
        }
    }

    winding_number
}

fn segment_winding_contribution(p1: Vec2, p2: Vec2, point: Vec2) -> i32 {
    if p1.y <= point.y {
        if p2.y > point.y && is_left(p1, p2, point) > 0.0 {
            // Upward crossing, and the point is to the left of the segment
            return 1;
        }
    } else {
        if p2.y <= point.y && is_left(p1, p2, point) < 0.0 {
            // Downward crossing, and the point is to the right of the segment
            return -1;
        }
    }
    0
}

fn is_left(p1: Vec2, p2: Vec2, point: Vec2) -> f64 {
    // Compute determinant to check if the point is to the left of the line
    (p2.x - p1.x) * (point.y - p1.y) - (point.x - p1.x) * (p2.y - p1.y)
}

pub fn rotate_vector(vector: Vec2, angle: f64) -> Vec2 {
    // Compute the cosine and sine of the angle
    let cos_theta = angle.cos();
    let sin_theta = angle.sin();

    // Apply the rotation formula
    Vec2::new(
        vector.x * cos_theta - vector.y * sin_theta,
        vector.x * sin_theta + vector.y * cos_theta,
    )
}

// Unit perpendicular of a segment formad by two points
pub fn unit_perpendicular(v1: Vec2, v2: Vec2, clockwise: bool) -> Option<Vec2> {
    // Vector representing the segment
    let v = v2 - v1;
    // Compute the perpendicular vector
    let perp = if clockwise {
        Vec2::new(-v.y, v.x) // Clockwise
    } else {
        Vec2::new(v.y, -v.x) // Counterclockwise
    };
    if perp.hypot() < EPSILON {
        log!("WARNING: Degenerate case unit_perpendicular()");
        return None;
    }
    // Normalize to unit length
    Some(perp.normalize())
}
// Project v1(pt2-pos) vector to the perpendicular of vector v2(pt1-pt2)
// pub fn get_unit_vector_perpendicular(pt1: Vec2, pt2: Vec2, pos: Vec2) -> Option<Vec2> {
//     let v1 = pos - pt2;
//     let v2 = pt2 - pt1;
//     if v2.hypot2() == 0. {
//         log!("WARNING: Degenerate case, get_unit_vector_perpendicular()");
//         return None;
//     }
//     let pv2 = Vec2::new(-v2.y, v2.x).normalize();
//     let dot_product = v1.dot(pv2) * pv2;

//     Some(dot_product)
// }

// Project v on v_p =(pt2-pt1)
pub fn project_on_vec(pt1: Vec2, pt2: Vec2, v: Vec2) -> Vec2 {
    let v_p = pt2 - pt1;
    let denom = v_p.dot(v_p);
    if denom.abs() < f64::EPSILON {
        return v;
    }
    v_p * v.dot(v_p) / denom
}

pub fn symmetric_point_to_segment(p1: Vec2, p2: Vec2, q: Vec2) -> Vec2 {
    // Segment midpoint
    let midpoint = (p1 + p2) / 2.0;

    if (p2 - p1).hypot() == 0.0 {
        log!("WARNING: Degenerate case symmetric_point_to_segment()");
        return p1;
    }

    // Segment direction vector
    let direction = (p2 - p1).normalize();

    // Vector from midpoint to the point `q`
    let to_point = q - midpoint;

    // Project `to_point` onto the segment's direction
    let projection_length = to_point.dot(direction);
    let projection = midpoint + projection_length * direction;

    // Calculate the symmetric point
    let symmetric = 2.0 * projection - q;

    symmetric
}

/// Returns `Some((distance, projection))` if the orthogonal projection of `q`
/// onto the infinite line through `p1->p2` lies within the segment [p1, p2].
/// Otherwise, returns `None`.
pub fn distance_and_projection_to_segment(
    p1: Vec2,
    p2: Vec2,
    q: Vec2,
    offset: f64,
) -> Option<(f64, Vec2)> {
    let seg = p2 - p1; // segment vector
    let pq = q - p1; // from p1 to q

    let seg_len_sq = seg.length_squared();
    if seg_len_sq < f64::EPSILON {
        // Degenerate case: segment is effectively a point.
        // Decide how you want to handle it; here, we say there's no valid 'segment'.
        return None;
    }

    // Parametric distance (t) along segment from p1:
    // t=0 => p1, t=1 => p2, <0 => before p1, >1 => beyond p2
    let t = pq.dot(seg) / seg_len_sq;

    // If outside [0..1], there's no valid orth projection within the segment
    if t < 0.0 || t > 1.0 {
        return None;
    }

    // The actual projection point on the segment
    let proj = p1 + t * seg;

    if (proj - p1).hypot() < offset || (proj - p2).hypot() < offset {
        return None;
    }
    // Distance from q to its projection
    let dist = (q - proj).hypot();

    Some((dist, proj))
}

/// Returns `Some((distance, projected_point))` if the angle is within
/// the arc’s sweep and *outside* the small "guard" zone around the start and end.
/// Otherwise returns `None`.
pub fn distance_and_projection_to_arc(
    arc: &Arc,
    pos: Vec2,
    guard: f64, // small angle in radians to exclude near start & end
) -> Option<(f64, Vec2)> {
    let cp = pos - arc.center.to_vec2();
    let dist_cp = cp.hypot();

    // Normalize angles to [0, 2π).
    let start = arc.start_angle.rem_euclid(2.0 * PI);
    let end = (arc.start_angle + arc.sweep_angle).rem_euclid(2.0 * PI);

    // Angle of the point w.r.t. the center, also in [0, 2π).
    let theta_p = cp.atan2().rem_euclid(2.0 * PI);

    // 1) Check if theta_p is in the arc sweep range
    let in_arc_range = |angle: f64| {
        if arc.sweep_angle >= 0.0 {
            // CCW arc
            if start <= end {
                // Normal range: start -> end
                angle >= start && angle <= end
            } else {
                // Arc spans the 2π boundary
                angle >= start || angle <= end
            }
        } else {
            // CW arc
            if end <= start {
                // Normal range: end -> start
                angle <= start && angle >= end
            } else {
                // Arc spans the 2π boundary in CW sense
                angle <= start || angle >= end
            }
        }
    };

    // 2) Helper to measure the minimal distance on a circle
    //    (so angle_dist(a, b) is how many radians separate angles a and b).
    let angle_dist = |a: f64, b: f64| {
        let d = (a - b).abs();
        d.min(2.0 * PI - d)
    };

    // 3) If the angle is in the arc’s sweep
    if in_arc_range(theta_p) {
        // but too close to start or end (within `guard` radians),
        // then we say "no valid projection" => return None.
        if angle_dist(theta_p, start) <= guard || angle_dist(theta_p, end) <= guard {
            return None;
        }

        // Otherwise, proceed with your existing distance/projection logic:
        let radius = arc.radii.x; // The relevant radius for this arc
        let dir = cp / dist_cp; // Unit direction from center to pos
        let proj_pt = arc.center + dir * radius;
        let dist = (dist_cp - radius).abs();

        Some((dist, proj_pt.to_vec2()))
    } else {
        None
    }
}

pub fn project_to_segment_with_direction(p1: Vec2, p2: Vec2, q: Vec2) -> (f64, f64) {
    // First segment vector
    let v1 = p2 - p1;

    // Second segment vector (q already starts at the origin)
    let v2 = q;

    // Dot product of v2 and v1
    let dot_product = v2.dot(v1);

    // Magnitude squared of v1
    let v1_magnitude_squared = v1.length_squared();

    // Projection formula
    let projection_length = dot_product / v1_magnitude_squared;
    let projection = projection_length * v1;

    // Direction relative to the first segment
    let direction = dot_product.signum(); // -1.0, 0.0, or 1.0

    (projection.hypot(), direction)
}

pub fn project_to_perpendicular(p1: Vec2, p2: Vec2, q: Vec2) -> (Vec2, f64) {
    // First segment vector
    let v1 = p2 - p1;

    // Perpendicular vector to the first segment
    let v_perp = Vec2::new(-v1.y, v1.x);

    // Second segment vector (q already starts at the origin)
    let v2 = q;

    // Dot product of v2 and v_perp
    let dot_product = v2.dot(v_perp);

    // Magnitude squared of v_perp
    let v_perp_magnitude_squared = v_perp.length_squared();

    // Projection formula
    (
        (dot_product / v_perp_magnitude_squared) * v_perp,
        dot_product.signum(),
    )
}

pub fn project_to_perpendicular_with_direction(p1: Vec2, p2: Vec2, q: Vec2) -> (f64, f64) {
    // First segment vector
    let v1 = p2 - p1;

    // Perpendicular vector to the first segment
    let v_perp = Vec2::new(-v1.y, v1.x);

    // Second segment vector (q already starts at the origin)
    let v2 = q;

    // Dot product of v2 and v_perp
    let dot_product = v2.dot(v_perp);

    // Magnitude squared of v_perp
    let v_perp_magnitude_squared = v_perp.length_squared();

    // Projection formula
    let projection = (dot_product / v_perp_magnitude_squared) * v_perp;

    // Direction relative to the first segment's perpendicular
    let direction = dot_product.signum(); // -1.0, 0.0, or 1.0

    (projection.hypot(), direction)
}

pub fn get_middle_from_start_end_positions(start: Vec2, end: Vec2, width: f64) -> Vec2 {
    let middle = (start + end) / 2.;
    let radius = width / 2.;
    let angle = (end - start).atan2();
    middle + Vec2::from_angle(angle + FRAC_PI_2) * radius
}

pub fn bez_path_to_geo_polygon(bez_path: &BezPath) -> Polygon<f64> {
    let mut points = Vec::new();
    for element in bez_path.elements() {
        match element {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => points.push((p.x, p.y)),
            PathEl::ClosePath => {
                // Le polygone doit être fermé
                if points.first() != points.last() {
                    points.push(points[0]);
                }
            }
            _ => log!("Error: Non-linear path elements found."),
        }
    }

    if points.len() < 3 {
        Polygon::new(LineString::new(vec![]), vec![])
    } else {
        Polygon::new(points.into(), vec![])
    }
}
pub fn geo_polygon_to_bez_path(polygon: &Polygon<f64>) -> Vec<BezPath> {
    let mut vec_bez_path = Vec::new();

    // Convert exterior ring
    if let Some(exterior_path) = ring_to_bez_path(polygon.exterior()) {
        vec_bez_path.push(exterior_path);
    }
    // Convert interior rings (holes)
    polygon.interiors().iter().for_each(|interior| {
        if let Some(interior_path) = ring_to_bez_path(interior) {
            vec_bez_path.push(interior_path);
        }
    });

    vec_bez_path
}

pub fn calc_segs(bez_path: BezPath) -> BezPath {
    let mut bez_path_flat = BezPath::new();
    flatten(bez_path, 0.15, |s| bez_path_flat.push(s));
    bez_path_flat
}

pub fn calc_polygon(bez_path_flat: &BezPath) -> Polygon<f64> {
    bez_path_to_geo_polygon(bez_path_flat)
}

// Helper function to convert a ring (either exterior or interior) to a bezier path
pub fn ring_to_bez_path(ring: &LineString<f64>) -> Option<BezPath> {
    let points: Vec<_> = ring
        .coords()
        .map(|coord| kurbo::Point::new(coord.x, coord.y))
        .collect();
    if points.len() < 2 {
        return None; // Skip invalid rings
    }

    let mut bez_path = BezPath::new();
    bez_path.push(PathEl::MoveTo(points[0]));
    points
        .iter()
        .skip(1)
        .for_each(|&point| bez_path.push(PathEl::LineTo(point)));

    // Close the path if it's a closed ring
    if points.first() == points.last() {
        bez_path.push(PathEl::ClosePath);
    }

    Some(bez_path)
}

pub fn is_near_line(point: Vec2, angle: f64, cursor: Vec2, precision: f64) -> bool {
    let dx = cursor.x - point.x;
    let dy = cursor.y - point.y;
    let distance = (dx * angle.sin() - dy * angle.cos()).abs();
    distance <= precision
}

// pub fn get_line_segment(size: &Size, point: Vec2, angle: f64) -> (Vec2, Vec2) {
//     let width = size.width;
//     let height = size.height;
//     if angle.abs() == FRAC_PI_2 {
//         let x = point.x;
//         return (Vec2::new(x, 0.), Vec2::new(x, height));
//     }
//     let m = angle.tan();
//     if m == 0. {
//         let y = point.y;
//         return (Vec2::new(0., y), Vec2::new(width, y));
//     }
//     // y= 0 intersection
//     let x0 = -point.y / m + point.x;
//     // y= height intersection
//     let xh = (height - point.y) / m + point.x;
//     // x= 0 intersection
//     let y0 = m * -point.x + point.y;
//     // x= width intersection
//     let yw = m * (width - point.x) + point.y;
//     let x0_inside = x0 >= 0. && x0 <= width;
//     let xh_inside = xh >= 0. && xh <= width;
//     let y0_inside = y0 >= 0. && y0 <= height;
//     let yw_inside = yw >= 0. && yw <= height;
//     match (x0_inside, xh_inside, y0_inside, yw_inside) {
//         (true, true, false, false) => (Vec2::new(x0, 0.), Vec2::new(xh, height)),
//         (false, false, true, true) => (Vec2::new(0., y0), Vec2::new(width, yw)),
//         (true, false, false, true) => (Vec2::new(x0, 0.), Vec2::new(width, yw)),
//         (true, false, true, false) => (Vec2::new(0., y0), Vec2::new(x0, 0.)),
//         (false, true, false, true) => (Vec2::new(width, y0), Vec2::new(xh, height)),
//         (false, true, true, false) => (Vec2::new(0., y0), Vec2::new(xh, height)),
//         _ => (Vec2::new(0., 0.), Vec2::new(width, height)),
//     }
// }

pub fn get_line_segment(size: &Size, point: Vec2, angle: f64) -> (Vec2, Vec2) {
    let width = size.width;
    let height = size.height;

    // Handle vertical lines (angle = ±π/2)
    if (angle - std::f64::consts::FRAC_PI_2).abs() < EPSILON
        || (angle + std::f64::consts::FRAC_PI_2).abs() < EPSILON
    {
        let x = point.x;
        return (Vec2::new(x, 0.0), Vec2::new(x, height));
    }

    // Handle horizontal lines (angle = 0 or π)
    let m = angle.tan();
    if m.abs() < EPSILON {
        let y = point.y;
        return (Vec2::new(0.0, y), Vec2::new(width, y));
    }

    // Calculate intersections
    let x0 = -point.y / m + point.x; // Intersection with y = 0
    let xh = (height - point.y) / m + point.x; // Intersection with y = height
    let y0 = m * -point.x + point.y; // Intersection with x = 0
    let yw = m * (width - point.x) + point.y; // Intersection with x = width

    // Collect valid points inside the canvas
    let mut intersections = vec![];
    if x0 >= 0.0 && x0 <= width {
        intersections.push(Vec2::new(x0, 0.0));
    }
    if xh >= 0.0 && xh <= width {
        intersections.push(Vec2::new(xh, height));
    }
    if y0 >= 0.0 && y0 <= height {
        intersections.push(Vec2::new(0.0, y0));
    }
    if yw >= 0.0 && yw <= height {
        intersections.push(Vec2::new(width, yw));
    }

    // Ensure two valid points
    if intersections.len() == 2 {
        (intersections[0], intersections[1])
    } else {
        // Fallback: Return a degenerate line if something goes wrong
        (Vec2::new(0.0, 0.0), Vec2::new(width, height))
    }
}

pub fn snap_pt(pos: Vec2, snap: f64) -> Vec2 {
    // Avoid division by zero
    if snap.abs() < EPSILON {
        return pos;
    }
    // Snap to grid
    let x = (pos.x / snap).round() * snap;
    let y = (pos.y / snap).round() * snap;
    Vec2::new(x, y)
}
pub fn snap_val(val: f64, snap: f64) -> f64 {
    // Avoid division by zero
    if snap.abs() < EPSILON {
        return val;
    }
    // Snap to grid
    let val = (val / snap).round() * snap;
    val
}

pub fn snap_angle_hv(angle: f64) -> f64 {
    let angle = angle % (2. * PI);
    if angle.abs_diff_eq(&0., THREAS_ANGLE) {
        return 0.;
    }
    if angle.abs_diff_eq(&PI, THREAS_ANGLE) {
        return PI;
    }
    if angle.abs_diff_eq(&FRAC_PI_2, THREAS_ANGLE) {
        return FRAC_PI_2;
    }
    if angle.abs_diff_eq(&-FRAC_PI_2, THREAS_ANGLE) {
        return -FRAC_PI_2;
    }
    angle
}

pub fn angle_from(v1: Vec2, v2: Vec2) -> f64 {
    let cross = v1.x * v2.y - v1.y * v2.x; // Cross product
    cross.atan2(v1.dot(v2)) // Returns the signed angle in radians
}
pub fn is_near_arc(start: Vec2, end: Vec2, radius: f64, pos: Vec2, precision: f64) -> bool {
    let center = (start + end) / 2.0;
    let angle = angle_from(pos - center, start - center);
    let dist = (pos - center).hypot();
    dist > radius - precision && dist < radius + precision && angle.abs() < std::f64::consts::PI
}

pub fn get_arc_center(start: Vec2, end: Vec2, radius: f64, up: bool) -> Vec2 {
    let chord_midpoint = (start + end) / 2.0;
    let chord_vector = end - start;

    // Handle degenerate case: start and end are the same
    if chord_vector.hypot() < f64::EPSILON {
        log!("WARNING: Degenerate case in get_arc_center()");
        return start;
    }

    let perpendicular = Vec2::new(-chord_vector.y, chord_vector.x).normalize();
    let chord_length = chord_vector.hypot() / 2.0;

    // Check if the radius is valid
    let radius_squared = radius.powi(2);
    let chord_length_squared = chord_length.powi(2);

    if radius_squared < chord_length_squared {
        log!("WARNING: Invalid radius. Radius is too small to form an arc.");
        return (start + end) / 2.0;
    }

    // Compute the distance from the midpoint to the center
    let height = (radius_squared - chord_length_squared).sqrt();

    if up {
        chord_midpoint + perpendicular * height
    } else {
        chord_midpoint - perpendicular * height
    }
}

pub fn find_circle_center(start: Vec2, end: Vec2, mut radius: f64, concavity: bool) -> Vec2 {
    // Vector of the chord
    let chord = end - start;
    let chord_length = chord.hypot();

    // Check if the radius is valid (must be >= half the chord length)
    if radius.abs() < chord_length / 2.0 {
        radius = radius.signum() * chord_length / 2.0;
    }

    // Midpoint of the chord
    let midpoint = (start + end) / 2.0;

    // Perpendicular vector to the chord
    let perpendicular = Vec2::new(-chord.y, chord.x).normalize();

    // Distance from the midpoint to the arc center
    let h = (radius.powi(2) - (chord_length / 2.0).powi(2)).sqrt();

    // Determine the arc center based on the radius' sign (concavity)
    match (concavity, radius > 0.0) {
        (true, true) => midpoint + perpendicular * h,
        (true, false) => midpoint - perpendicular * h,
        (false, true) => midpoint - perpendicular * h,
        (false, false) => midpoint + perpendicular * h,
    }
}
/// Creates an arc passing through `start` and `end` with the specified `radius`.
/// The `concavity` parameter determines whether the arc is the smallest (true) or the largest (false).
/// The radius sign determines the arc's direction (concavity up/down).
pub fn create_arc_from_radius_and_concavity(
    start: Vec2,
    end: Vec2,
    radius: f64,
    concavity: bool,
) -> Arc {
    let arc_center = find_circle_center(start, end, radius, concavity);

    // Start and end angles of the arc
    let start_angle = (start - arc_center).atan2();
    let end_angle = (end - arc_center).atan2();

    // Sweep angle
    let mut sweep_angle = end_angle - start_angle;

    if concavity ^ (radius > 0.0) {
        // Smallest arc: Adjust sweep angle to be less than 180 degrees
        if radius > 0.0 && sweep_angle < 0.0 {
            sweep_angle += 2.0 * std::f64::consts::PI;
        } else if radius < 0.0 && sweep_angle > 0.0 {
            sweep_angle -= 2.0 * std::f64::consts::PI;
        }
    } else {
        // Largest arc: Adjust sweep angle to be more than 180 degrees
        if radius > 0.0 && sweep_angle > 0.0 {
            sweep_angle -= 2.0 * std::f64::consts::PI;
        } else if radius < 0.0 && sweep_angle < 0.0 {
            sweep_angle += 2.0 * std::f64::consts::PI;
        }
    }

    Arc {
        center: arc_center.to_point(),
        radii: Vec2::new(radius.abs(), radius.abs()), // Uniform radius
        start_angle,
        sweep_angle,
        x_rotation: 0.0, // No rotation in this implementation
    }
}

pub fn is_point_near_arc(arc: &Arc, point: Vec2, tolerance: f64) -> bool {
    // Calculate the center-to-point vector
    let center_to_point = point - arc.center.to_vec2();

    // Calculate the distance from the center to the point
    let distance = center_to_point.hypot();

    // Check if the distance is within the radius ± tolerance
    if (distance - arc.radii.x).abs() > tolerance {
        return false;
    }

    // Calculate the angle from the center to the point and normalize to [0, 2π]
    let point_angle = center_to_point
        .atan2()
        .rem_euclid(2.0 * std::f64::consts::PI);

    // Normalize the arc's start and end angles to [0, 2π]
    let start_angle = arc.start_angle.rem_euclid(2.0 * std::f64::consts::PI);
    let end_angle = (arc.start_angle + arc.sweep_angle).rem_euclid(2.0 * std::f64::consts::PI);

    // Check if the point's angle is within the arc's angle range
    if arc.sweep_angle > 0.0 {
        // Counterclockwise arc
        if start_angle <= end_angle {
            point_angle >= start_angle && point_angle <= end_angle
        } else {
            // Wrapped around 0
            point_angle >= start_angle || point_angle <= end_angle
        }
    } else {
        // Clockwise arc
        if end_angle <= start_angle {
            point_angle <= start_angle && point_angle >= end_angle
        } else {
            // Wrapped around 0
            point_angle <= start_angle || point_angle >= end_angle
        }
    }
}

/// Get the middle point of an arc
pub fn middle_point_of_arc(arc: &Arc) -> Vec2 {
    // Calculate the mid-angle
    let mid_angle = arc.start_angle + arc.sweep_angle / 2.0;

    // Calculate the x and y coordinates of the middle point
    Vec2::new(
        arc.center.x + mid_angle.cos() * arc.radii.x,
        arc.center.y + mid_angle.sin() * arc.radii.y,
    )
}

pub fn get_arc_radius(start: Vec2, end: Vec2, current_radius: f64, pos: Vec2, up: bool) -> f64 {
    // Compute the current arc center
    let current_center = get_arc_center(start, end, current_radius, up);
    // Compute the new radius based on the cursor's distance to the current center
    (pos - current_center).hypot()
}

pub fn move_arc_middle(start: Vec2, end: Vec2, old_center: Vec2, new_middle: Vec2) -> Vec2 {
    // Compute the chord midpoint (old middle point)
    let old_middle = (start + end) / 2.0;

    // Compute the vector from the old middle to the new middle
    let adjustment = new_middle - old_middle;

    // Adjust the old center by the same adjustment vector
    let new_center = old_center + adjustment;

    // Ensure the radius remains constant
    let radius = (start - old_center).hypot();
    let current_radius = (start - new_center).hypot();
    if (current_radius - radius).abs() > EPSILON {
        log!("Radius has changed! Ensure the new middle point is valid.");
        return old_center;
    }

    new_center
}

pub fn perpendicular_point(start: Vec2, end: Vec2, dist: f64, up: bool) -> Vec2 {
    // Midpoint of the segment
    let midpoint = (start + end) / 2.0;

    // Vector of the segment
    let segment_vector = end - start;

    if segment_vector.hypot() < EPSILON {
        log!("WARNING: Degenerate case, perpendicular_point()");
        return start;
    }

    // Perpendicular vectors
    let perp = if up {
        Vec2::new(-segment_vector.y, segment_vector.x).normalize()
    } else {
        Vec2::new(segment_vector.y, -segment_vector.x).normalize()
    };

    midpoint + perp * dist
}

pub fn perpendicular_point_with_projection(start: Vec2, end: Vec2, dpos: Vec2, up: bool) -> Vec2 {
    // Midpoint of the segment
    let midpoint = (start + end) / 2.0;

    // Vector of the segment

    let segment_vector = end - start; // if up { end - start } else { start - end };

    // Handle degenerate case: start and end are the same
    if segment_vector.hypot() < f64::EPSILON {
        log!("WARNING: Degenerate case, perpendicular_point_with_projection()");
        return start;
    }

    // Perpendicular vectors
    let perp = if up {
        Vec2::new(-segment_vector.y, segment_vector.x).normalize()
    } else {
        Vec2::new(segment_vector.y, -segment_vector.x).normalize()
    };

    // Project dpos onto the perpendicular vector
    let projection = dpos.dot(perp);

    // Move the midpoint by the projected distance along the perpendicular
    midpoint + perp * projection
}

// pub fn create_arc_from_center(start: Vec2, end: Vec2, center: Vec2, sweep_sign: bool) -> Arc {
//     // Calculate the radius
//     let radius = (start - center).hypot();
//     // Calculate the start and end angles
//     let start_angle = (start - center).atan2();
//     let end_angle = (end - center).atan2();
//     // Calculate the sweep angle based on the concavity
//     let sweep_angle = if sweep_sign {
//         (end_angle - start_angle).rem_euclid(2.0 * std::f64::consts::PI)
//     } else {
//         (start_angle - end_angle).rem_euclid(2.0 * std::f64::consts::PI) * -1.0
//     };
//     // Construct the arc
//     Arc {
//         center: center.to_point(),
//         radii: Vec2::new(radius, radius),
//         start_angle,
//         sweep_angle,
//         x_rotation: 0.0,
//     }
// }

pub fn perpendicular_points_with_distance(
    pt1: Vec2,
    pt2: Vec2,
    dist: f64,
    concavity: bool,
) -> Vec2 {
    // Compute the vector of the segment
    let segment_vector = pt2 - pt1;
    let segment_length = segment_vector.hypot();

    // Compute the midpoint of the segment
    let midpoint = (pt1 + pt2) / 2.0;
    // Ensure dist is valid
    if dist < segment_length / 2.0 {
        log!("WARNING: Invalid distance in perpendicular_points_with_distance()");
        return midpoint;
    }

    // Perpendicular vector (normalized)
    let perpendicular = Vec2::new(-segment_vector.y, segment_vector.x).normalize();

    // Two points on the perpendicular bisector at the given distance
    if concavity {
        midpoint + perpendicular * dist
    } else {
        midpoint - perpendicular * dist
    }
}

pub fn line_line_intersection(
    point1: Vec2,
    angle1: f64,
    point2: Vec2,
    angle2: f64,
) -> Option<Vec2> {
    // Direction vectors for the two lines
    let dir1 = Vec2::new(angle1.cos(), angle1.sin());
    let dir2 = Vec2::new(angle2.cos(), angle2.sin());

    // Differences between the points
    let delta = point2 - point1;

    // Determinant (cross product of direction vectors)
    let det = dir1.x * dir2.y - dir1.y * dir2.x;

    // If determinant is zero, the lines are parallel or coincident
    if det.abs() < f64::EPSILON {
        return None;
    }

    // Solve for t and u
    let t = (delta.x * dir2.y - delta.y * dir2.x) / det;

    // Compute the intersection point
    Some(point1 + dir1 * t)
}

/// Find the intersection points of a line (origin, angle) and a circle (center, radius)
pub fn line_circle_intersection_with_angle(
    origin: Vec2,
    angle: f64,
    center: Vec2,
    radius: f64,
) -> Option<(Vec2, Option<Vec2>)> {
    // Line direction vector from the angle
    let direction = Vec2::new(angle.cos(), angle.sin());
    // Vector from the circle center to the line origin
    let f = origin - center;
    // Quadratic coefficients
    let a = direction.dot(direction);
    let b = 2.0 * f.dot(direction);
    let c = f.dot(f) - radius.powi(2);
    // Discriminant
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        // No intersection
        None
    } else {
        let discriminant_sqrt = discriminant.sqrt();
        let t1 = (-b - discriminant_sqrt) / (2.0 * a);
        let t2 = (-b + discriminant_sqrt) / (2.0 * a);

        // Calculate intersection points
        match (t1.is_finite(), t2.is_finite()) {
            (true, true) => {
                if (t1 - t2).abs() < EPSILON {
                    Some((origin + direction * t1, None))
                } else {
                    Some((origin + direction * t1, Some(origin + direction * t2)))
                }
            }
            (true, false) => Some((origin + direction * t1, None)),
            (false, true) => Some((origin + direction * t2, None)),
            (false, false) => None,
        }
    }
}

/// Find the intersection points of two circles
pub fn circle_circle_intersection(
    center1: Vec2,
    radius1: f64,
    center2: Vec2,
    radius2: f64,
) -> Option<(Vec2, Option<Vec2>)> {
    let d = (center2 - center1).hypot();
    // No intersection cases
    if d > radius1 + radius2 || d < (radius1 - radius2).abs() || d.abs() < f64::EPSILON {
        return None;
    }
    let a = (radius1.powi(2) - radius2.powi(2) + d.powi(2)) / (2.0 * d);
    let h = (radius1.powi(2) - a.powi(2)).sqrt();
    let p2 = Point::new(
        center1.x + a * (center2.x - center1.x) / d,
        center1.y + a * (center2.y - center1.y) / d,
    );
    let intersection1 = Vec2::new(
        p2.x + h * (center2.y - center1.y) / d,
        p2.y - h * (center2.x - center1.x) / d,
    );
    let intersection2 = Vec2::new(
        p2.x - h * (center2.y - center1.y) / d,
        p2.y + h * (center2.x - center1.x) / d,
    );
    if intersection1 == intersection2 {
        Some((intersection1, None))
    } else {
        Some((intersection1, Some(intersection2)))
    }
}

pub fn get_point_at_dist_from_angle(origin: Vec2, angle: f64, dist: f64) -> Vec2 {
    origin + Vec2::new(angle.cos(), angle.sin()) * dist
}

/// Splits an arc (defined by two points, a radius, and concavity) with a circle centered at `start`
/// returning two arcs if there are two valid intersection points.
pub fn split_arc_with_circle(
    start: Vec2,
    end: Vec2,
    arc_radius: f64,
    arc_concavity: bool,
    circle_radius: f64,
) -> Option<(Arc, Arc)> {
    // 1. Compute the arc center and angles for the original arc
    let arc_center = get_arc_center(start, end, arc_radius, arc_concavity);
    let start_angle = (start - arc_center).atan2();
    let end_angle = (end - arc_center).atan2();

    // Sweep angle (handle concavity if needed)
    // For a "simple" approach, we do:
    //   sweep_angle = end_angle - start_angle
    // But you can adjust for concavity if the sign is inverted
    let mut sweep_angle = end_angle - start_angle;

    // Normalize sweep angle to range (-π..π) or [0..2π),
    // ensuring correct direction for "concavity".
    // A simple approach is to keep the sign if arc_concavity is relevant:
    if arc_concavity && sweep_angle > 0.0 {
        sweep_angle -= 2.0 * std::f64::consts::PI;
    } else if !arc_concavity && sweep_angle < 0.0 {
        sweep_angle += 2.0 * std::f64::consts::PI;
    }

    // Construct the original arc
    let _original_arc = Arc {
        center: arc_center.to_point(),
        radii: Vec2::new(arc_radius, arc_radius),
        start_angle,
        sweep_angle,
        x_rotation: 0.0,
    };

    // 2. The circle is centered at `start`, radius = `circle_radius`
    // We'll find intersections between that circle and the arc's circle
    let circle_center = start;
    let circle_r = circle_radius;

    // 3. Find intersection points between the two circles
    match find_arc_circle_intersections(arc_center, arc_radius, circle_center, circle_r) {
        None => None, // No intersection or infinite intersection
        Some((_pt1, None)) => {
            // Only one intersection => tangent
            // The arc is not really "split" into two arcs,
            // but you could return a degenerate second arc if desired.
            // For now, we treat it as no valid two-arc split.
            None
        }
        Some((pt1, Some(pt2))) => {
            // Two intersection points: split into two arcs

            // Convert intersection points to angles relative to arc_center
            let angle1 = (pt1 - arc_center).atan2();
            let angle2 = (pt2 - arc_center).atan2();

            // Sort angles so arc1 is [start_angle..angle1], arc2 is [angle1..end_angle]
            let (mid_angle1, _mid_angle2) = if angle1 < angle2 {
                (angle1, angle2)
            } else {
                (angle2, angle1)
            };

            // Ensure mid angles lie within [start_angle..start_angle+sweep_angle]
            // or handle them as is if you assume geometry is correct

            // Arc 1: from start_angle to mid_angle1
            let arc1 = Arc {
                center: arc_center.to_point(),
                radii: Vec2::new(arc_radius, arc_radius),
                start_angle,
                sweep_angle: mid_angle1 - start_angle,
                x_rotation: 0.0,
            };

            // Arc 2: from mid_angle1 to end_angle
            let arc2 = Arc {
                center: arc_center.to_point(),
                radii: Vec2::new(arc_radius, arc_radius),
                start_angle: mid_angle1,
                sweep_angle: (start_angle + sweep_angle) - mid_angle1,
                x_rotation: 0.0,
            };

            Some((arc1, arc2))
        }
    }
}

/// Finds intersection points between two circles:
/// Circle1: center=arc_center, radius=arc_radius
/// Circle2: center=circle_center, radius=circle_radius
/// Returns:
///   None => No intersection or infinite intersection (coincident)
///   Some((pt1, None)) => tangent
///   Some((pt1, Some(pt2))) => two intersections
fn find_arc_circle_intersections(
    arc_center: Vec2,
    arc_radius: f64,
    circle_center: Vec2,
    circle_radius: f64,
) -> Option<(Vec2, Option<Vec2>)> {
    // Distance between centers
    let d = (circle_center - arc_center).hypot();
    // Check for no intersection
    if d > arc_radius + circle_radius
        || d < (arc_radius - circle_radius).abs()
        || d.abs() < f64::EPSILON
    {
        return None;
    }

    // Solve circle-circle intersection
    let a = (arc_radius.powi(2) - circle_radius.powi(2) + d.powi(2)) / (2.0 * d);
    let h = (arc_radius.powi(2) - a.powi(2)).sqrt();

    // Midpoint of intersection chord
    let mid = arc_center + (circle_center - arc_center) * (a / d);

    let offset = circle_center - arc_center;
    let offset_perp = Vec2::new(-offset.y, offset.x);

    let i1 = mid + offset_perp * (h / d);
    let i2 = mid - offset_perp * (h / d);

    if h.abs() < f64::EPSILON {
        // One intersection => tangent
        Some((i1, None))
    } else {
        Some((i1, Some(i2)))
    }
}

/// Normalize an angle to the range [0, 2π)
fn _normalize_angle(angle: f64) -> f64 {
    angle.rem_euclid(2.0 * std::f64::consts::PI)
}

pub fn point_from_start(start: Vec2, end: Vec2, rad: f64) -> Vec2 {
    if rad < EPSILON {
        return start;
    }
    let seg = end - start;
    let dist = seg.hypot();

    if dist.abs() < f64::EPSILON {
        return start;
    }

    let dir = seg / dist;

    // No clamping here
    start + dir * rad
}

pub fn point_from_end(start: Vec2, end: Vec2, rad: f64) -> Vec2 {
    if rad < EPSILON {
        return end;
    }
    let seg = start - end;
    let dist = seg.hypot();

    if dist.abs() < f64::EPSILON {
        return start;
    }

    let dir = seg / dist;

    // No clamping here
    end + dir * rad
}

pub fn bisector_dir(a: Vec2, b: Vec2, c: Vec2) -> Vec2 {
    // 1. Vectors from b
    let v1 = a - b; // BA
    let v2 = c - b; // BC

    // Handle degenerate: if either vector is near zero, angle is not well-defined
    if v1.hypot() < f64::EPSILON || v2.hypot() < f64::EPSILON {
        return b;
    }

    // 2. Normalize
    let n1 = v1.normalize();
    let n2 = v2.normalize();

    // 3. Sum the normalized directions => angle bisector direction
    let bisector_dir = n1 + n2;

    // Degenerate case: if n1 ~ -n2 => A,B,C are ~180°, the "internal bisector" is ill-defined
    if bisector_dir.hypot() < f64::EPSILON {
        return b;
    }

    bisector_dir.normalize()
}

// Angle at b
pub fn project_onto_bisector(a: Vec2, b: Vec2, c: Vec2, p: Vec2) -> (Vec2, f64) {
    // Compute bisector
    let bisec = bisector_dir(a, b, c);

    // Compute projected point
    let t = p.dot(bisec);
    let p_proj = bisec * t;

    (p_proj, t.signum())
}

pub fn point_on_bisector(a: Vec2, b: Vec2, c: Vec2) -> (Vec2, f64) {
    // Angle
    let mut angle = angle_from(a - b, c - b) * 0.5;
    if (angle - PI / 2.).abs() < EPSILON || (angle + PI / 2.).abs() < EPSILON {
        return (b, 0.);
    }
    let sign = angle.signum();
    angle = angle.abs();

    let radius = (a - b).hypot() * angle.tan();
    let point = bisector_dir(a, b, c) * radius / angle.abs().sin();
    (point + b, -sign)
}

fn snap_length(length: f64, snap: f64) -> f64 {
    (length / snap).round() * snap
}

pub fn move_b_with_snapping(a: Vec2, b: Vec2, c: Vec2, dpos: Vec2, snap: f64) -> Vec2 {
    // Apply the small displacement to B
    let new_b = b + dpos;

    // Compute snapped lengths for AB and BC
    let ab = (new_b - a).hypot();
    let bc = (new_b - c).hypot();
    let snap_ab = snap_length(ab, snap);
    let snap_bc = snap_length(bc, snap);

    // Find the intersection of the two circles
    match circle_circle_intersection(a, snap_ab, c, snap_bc) {
        Some((i1, None)) => i1,
        Some((i1, Some(i2))) => {
            // Choose the intersection point closest to the proposed B
            let dist1 = (i1 - new_b).hypot();
            let dist2 = (i2 - new_b).hypot();
            if dist1 < dist2 {
                i1
            } else {
                i2
            }
        }
        _ => {
            log!("WARNING: No intersection found in move_b_with_snapping()");
            b
        }
    }
}

pub fn snap_end_to_multiple_of(start: Vec2, end: Vec2, dpos: Vec2, snap: f64) -> Vec2 {
    // 1. Nouvelle extrémité brute
    let new_end = end + dpos;

    // 2. Vecteur direction à partir de start
    let direction = new_end - start;

    // 3. Longueur actuelle
    let current_length = direction.hypot(); // ou direction.length()

    // Vérification pour éviter la division par zéro
    if current_length.abs() < f64::EPSILON {
        // Si le vecteur est (quasi) nul, on ne peut pas le "scaler" correctement
        return new_end;
    }

    // 4. Calcul de la longueur "snapée"
    let snapped_length = (current_length / snap).round() * snap;

    // 5. Facteur d'échelle
    let scale = if snapped_length.abs() < f64::EPSILON {
        0.0
    } else {
        snapped_length / current_length
    };

    // 6. Nouvelle fin "snapée"
    let snapped_end = start + direction * scale;

    snapped_end
}

pub fn get_coordinates_in_base(v1: Vec2, v2: Vec2, p: Vec2) -> (f64, f64) {
    let det = v1.x * v2.y - v1.y * v2.x;

    // Check if the vectors are linearly independent
    if det.abs() < EPSILON {
        return (0., 0.); // The base is not valid
    }

    // Compute the inverse of the base matrix
    let inv_mat = [[v2.y / det, -v2.x / det], [-v1.y / det, v1.x / det]];

    // Multiply the inverse matrix by the point `p`
    let a = inv_mat[0][0] * p.x + inv_mat[0][1] * p.y;
    let b = inv_mat[1][0] * p.x + inv_mat[1][1] * p.y;

    (a, b)
}
