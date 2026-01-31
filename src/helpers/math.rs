macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into())
    }
}
use crate::types::others::{SegBundle, Snap};
use approx::*;
use geo::algorithm::orient::Orient;
use geo::orient::Direction;
use geo::{LineString, MultiPolygon, Polygon};

use kurbo::{
    flatten, Arc, BezPath, Line, ParamCurveNearest, PathEl, Point, RoundedRectRadii, Size, Vec2,
};

use std::f64::consts::PI;
use std::f64::consts::*;

pub fn between(pos1: &Vec2, pos2: &Vec2) -> Vec2 {
    let diff = *pos2 - *pos1;
    Vec2::new(
        pos1.x.min(pos2.x) + diff.x / 3.,
        pos1.y.min(pos2.y) + diff.y / 3.,
    )
}

pub const EPSILON: f64 = 1e-6;

pub fn get_magnets_vertices(center_pt: Vec2, radius_pt: Vec2, count: usize) -> Vec<Vec2> {
    let mut vs = vec![];
    let start_angle = (radius_pt - center_pt).atan2();
    let radius = (radius_pt - center_pt).hypot();
    let step = 2.0 * PI / count as f64;
    for i in 0..count {
        let angle = start_angle + step * i as f64;
        let pos = center_pt + Vec2::new(radius * angle.cos(), radius * angle.sin());
        vs.push(pos);
    }
    vs
}
pub fn length_unit_to_mm(unit: &str) -> Option<f64> {
    match unit {
        "mm" => Some(1.0),
        "cm" => Some(10.0),
        "in" => Some(25.4),
        "px" => Some(1.0),
        _ => None,
    }
}

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
    true
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

    2.0 * projection - pos
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

    midpoint + projection_length * perpendicular
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

    midpoint + distance * perpendicular
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
    } else if p2.y <= point.y && is_left(p1, p2, point) < 0.0 {
        // Downward crossing, and the point is to the right of the segment
        return -1;
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

    2.0 * projection - q
}

/// Returns `Some((distance, projection point))` if the orthogonal projection of `q`
/// onto the infinite line through `p1->p2` lies within the segment [p1, p2].
/// Otherwise, returns `None`. The `offset` parameter is a small distance
/// to exclude points near the segment's endpoints.
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
    if !(0.0..=1.0).contains(&t) {
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
    let mut bez_path_flat = BezPath::new();
    flatten(bez_path, 0.15, |seg| bez_path_flat.push(seg));

    let mut points = Vec::new();
    let mut current_path_started = false;

    for element in bez_path_flat.elements() {
        match element {
            PathEl::MoveTo(p) => {
                if current_path_started {
                    // Ignore new subpaths
                    break;
                }
                points.push((p.x, p.y));
                current_path_started = true;
            }
            PathEl::LineTo(p) => {
                if current_path_started {
                    points.push((p.x, p.y));
                }
            }
            PathEl::ClosePath => {
                if !points.is_empty() && points.first() != points.last() {
                    points.push(points[0]);
                }
            }
            _ => {} // Should not happen after flatten()
        }
    }

    if points.len() < 3 {
        Polygon::new(LineString::new(vec![]), vec![])
    } else {
        Polygon::new(points.into(), vec![]).orient(Direction::Default)
    }
}

pub fn geo_multipolygon_to_bez_paths(multi: &MultiPolygon<f64>) -> Vec<BezPath> {
    let mut paths = Vec::new();
    for poly in multi.iter() {
        // exterior
        if let Some(p) = ring_to_bez_path(poly.exterior()) {
            paths.push(p);
        }
        // interiors (holes)
        for inner in poly.interiors() {
            if let Some(p) = ring_to_bez_path(inner) {
                paths.push(p);
            }
        }
    }
    paths
}

// Helper function to convert a ring (either exterior or interior) to a bezier path
pub fn ring_to_bez_path(ring: &LineString<f64>) -> Option<BezPath> {
    let pts: Vec<_> = ring.coords().map(|c| kurbo::Point::new(c.x, c.y)).collect();
    if pts.len() < 2 {
        return None;
    }

    let is_closed = pts.first() == pts.last();
    let take_len = if is_closed && pts.len() > 1 {
        pts.len() - 1
    } else {
        pts.len()
    };

    let mut path = BezPath::new();
    path.push(PathEl::MoveTo(pts[0]));
    for &p in &pts[1..take_len] {
        path.push(PathEl::LineTo(p));
    }
    if is_closed {
        path.push(PathEl::ClosePath);
    }
    Some(path)
}

// pub fn is_near_line(point: Vec2, angle: f64, cursor: Vec2, precision: f64) -> bool {
//     let dx = cursor.x - point.x;
//     let dy = cursor.y - point.y;
//     let distance = (dx * angle.sin() - dy * angle.cos()).abs();
//     distance <= precision
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

pub fn snap_vertex(pos: Vec2, snap: Snap) -> Vec2 {
    // Snap to grid
    let x = (pos.x / snap.linear()).round() * snap.linear();
    let y = (pos.y / snap.linear()).round() * snap.linear();
    Vec2::new(x, y)
}
pub fn snap_val(val: f64, snap: Snap) -> f64 {
    (val / snap.linear()).round() * snap.linear()
}
// Angle in degrees
pub fn snap_angle_deg(a: f64, snap: Snap) -> f64 {
    (a / snap.angle()).round() * snap.angle()
}
// Angle in radians
pub fn snap_angle(a: f64, snap: Snap) -> f64 {
    (a / PI * 180. / snap.angle()).round() * snap.angle() / 180. * PI
}

pub fn angle_from(v1: Vec2, v2: Vec2) -> f64 {
    let cross = v1.x * v2.y - v1.y * v2.x; // Cross product
    cross.atan2(v1.dot(v2)) // Returns the signed angle in radians
}
//

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

pub fn lines_intersection_1(point1: Vec2, angle1: f64, point2: Vec2, angle2: f64) -> Option<Vec2> {
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
pub fn lines_intersection_2(point1: Vec2, dir1: Vec2, point2: Vec2, dir2: Vec2) -> Option<Vec2> {
    // Differences between the points
    let delta = point2 - point1;

    // Determinant (cross product of the direction vectors)
    let det = dir1.x * dir2.y - dir1.y * dir2.x;

    // If determinant is zero (or near zero), the lines are parallel or coincident
    if det.abs() < f64::EPSILON {
        return None;
    }

    // Solve for t (the scalar parameter along dir1)
    let t = (delta.x * dir2.y - delta.y * dir2.x) / det;

    // Compute the intersection point
    Some(point1 + dir1 * t)
}

/// Computes the intersection point of two line segments, if it exists.
///
/// The first segment is defined by the points `p` and `p2`, and the second segment by the points `q` and `q2`.
/// Each segment is considered as a closed interval, meaning that the endpoints are included in the segment.
///
/// The function uses a parametric representation of the segments:
///
/// ```ignore
/// p + t * (p2 - p)
/// q + u * (q2 - q)
/// ```
///
/// and solves for the parameters `t` and `u` such that:
///
/// ```ignore
/// p + t * (p2 - p) = q + u * (q2 - q)
/// ```
///
/// The cross product `r.cross(s)` (where `r = p2 - p` and `s = q2 - q`) is used to determine if the segments
/// are parallel (or nearly collinear). If `rxs.abs()` is less than `EPSILON`, the segments are considered parallel,
/// and the function returns `None`.
///
/// If the segments are not parallel, `t` and `u` are computed as:
///
/// - `t = (q - p).cross(s) / r.cross(s)`
/// - `u = (q - p).cross(r) / r.cross(s)`
///
/// The segments intersect if and only if both `t` and `u` lie within the range `[0.0, 1.0]`.
/// In that case, the intersection point is given by `p + t * (p2 - p)`.
///
/// # Parameters
///
/// - `p`: The starting point of the first segment.
/// - `p2`: The ending point of the first segment.
/// - `q`: The starting point of the second segment.
/// - `q2`: The ending point of the second segment.
///
/// # Returns
///
/// - `Some(Vec2)` containing the intersection point if the segments intersect.
/// - `None` if the segments do not intersect, or if they are parallel/collinear within the tolerance defined by `EPSILON`.
///
/// # Example
///
/// ```
/// let p = Vec2::new(0.0, 0.0);
/// let p2 = Vec2::new(4.0, 4.0);
/// let q = Vec2::new(0.0, 4.0);
/// let q2 = Vec2::new(4.0, 0.0);
///
/// if let Some(intersection) = segment_intersection(p, p2, q, q2) {
///     println!("Intersection at: {:?}", intersection);
/// } else {
///     println!("No intersection.");
/// }
/// ```
///
pub fn segment_intersection(p: Vec2, p2: Vec2, q: Vec2, q2: Vec2) -> Option<Vec2> {
    let r = p2 - p;
    let s = q2 - q;
    let rxs = r.cross(s);
    let q_minus_p = q - p;

    // Check if the segments are parallel (or collinear).
    if rxs.abs() < EPSILON {
        return None;
    }

    let t = q_minus_p.cross(s) / rxs;
    let u = q_minus_p.cross(r) / rxs;

    // If t and u are between 0 and 1, the segments intersect.
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some(p + r * t)
    } else {
        None
    }
}

/// Computes the intersection points between a segment (from `p` to `p2`) and a circle
/// (with center `center` and radius `radius`).
///
/// Returns a vector containing zero, one, or two points.
/// Note that only intersections that occur within the segment (i.e. where t is between 0 and 1)
/// are included.
pub fn segment_circle_intersections(p: Vec2, p2: Vec2, center: Vec2, radius: f64) -> Vec<Vec2> {
    // The vector along the segment.
    let d = p2 - p;
    // The vector from the circle's center to the start of the segment.
    let f = p - center;

    // Quadratic coefficients for the equation:
    //   a*t^2 + b*t + c = 0
    let a = d.dot(d);
    let b = 2.0 * f.dot(d);
    let c = f.dot(f) - radius * radius;

    // The discriminant of the quadratic equation.
    let discriminant = b * b - 4.0 * a * c;

    // If the discriminant is negative, there are no real intersections.
    if discriminant < 0.0 {
        Vec::new()
    } else {
        let mut intersections = Vec::new();
        let discriminant_sqrt = discriminant.sqrt();
        // Find the two potential intersection parameters.
        let t1 = (-b - discriminant_sqrt) / (2.0 * a);
        let t2 = (-b + discriminant_sqrt) / (2.0 * a);

        // Only add the intersection if t is within [0, 1] (i.e. on the segment)
        if (0.0..=1.0).contains(&t1) {
            intersections.push(p + d * t1);
        }
        // t2 is different from t1 when discriminant > 0. For a tangent (discriminant == 0),
        // it would be the same, so we avoid a duplicate.
        if discriminant > 0.0 && (0.0..=1.0).contains(&t2) {
            intersections.push(p + d * t2);
        }
        intersections
    }
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

/// Moves the apex point with snapping. Given points `a` and `c` (the two fixed endpoints),
/// the original apex, and a displacement `dpos`, this function computes the new apex position
/// by snapping the distances |AB| and |BC| to multiples of `snap` and then finding the circle‐
/// circle intersection.
///
/// Returns `Some(new_apex)` if an intersection is found, or `None` if no valid intersection exists.
pub fn move_vertex_with_3v_snapping(
    a: Vec2,
    apex: Vec2,
    c: Vec2,
    dpos: Vec2,
    snap: f64,
) -> Option<Vec2> {
    // Apply the small displacement to the apex (B)
    let new_b = apex + dpos;

    // Compute the current distances from the new apex to points a and c
    let ab = (new_b - a).hypot();
    let bc = (new_b - c).hypot();

    // Snap these distances to the nearest multiple of 'snap'
    let snap_ab = (ab / snap).round() * snap;
    let snap_bc = (bc / snap).round() * snap;

    // Find the intersection of the two circles centered at 'a' and 'c'
    // with radii 'snap_ab' and 'snap_bc' respectively.
    match circle_circle_intersection(a, snap_ab, c, snap_bc) {
        // If there's a single intersection, return it.
        Some((i1, None)) => Some(i1),
        // If there are two intersections, choose the one closest to the new_b position.
        Some((i1, Some(i2))) => {
            let dist1 = (i1 - new_b).hypot();
            let dist2 = (i2 - new_b).hypot();
            if dist1 < dist2 {
                Some(i1)
            } else {
                Some(i2)
            }
        }
        // If no intersection is found, log a warning and return None.
        _ => {
            log!("WARNING: No intersection found in move_apex_with_snapping()");
            None
        }
    }
}

// pub fn area_from_hes(pts: &VecRing<HalfEdge>) -> f64 {
//     let len = pts.len();
//     let mut area = 0.0;
//     for idx in 0..len as i64 {
//         let vi = pts.get(idx).get_vertex().curr;
//         let vj = pts.get((idx + 1) % len as i64).get_vertex().curr;
//         area += vi.x * vj.y - vj.x * vi.y;
//     }
//     area * 0.5
// }

/// CORRECTED VERSION WITH Y AXIS INVERTED
pub fn arc_intersection_with_circle(arc: Arc, r: f64, circle_from_end: bool) -> Option<Vec2> {
    // Let arc_c be the center of the arc's circle and arc_r its radius.
    let arc_c = arc.center.to_vec2();
    let arc_r = arc.radii.x;

    // Choose the base angle depending on the flag.
    let base_angle = if circle_from_end {
        arc.start_angle + arc.sweep_angle
    } else {
        arc.start_angle
    };

    // Compute the circle's center: either the arc's start or end point.
    // Here we assume that the arc's angle is given in the y-down system.
    let circ_c = arc_c + Vec2::new(arc_r * base_angle.cos(), arc_r * base_angle.sin());

    // Find intersections between the arc's circle (center arc_c, radius arc_r) and the circle
    // centered at circ_c with radius r.
    let d = (arc_c - circ_c).length();
    if d > arc_r + r || d < (arc_r - r).abs() {
        return None;
    }

    // Standard circle–circle intersection formula.
    let a = (arc_r * arc_r - r * r + d * d) / (2.0 * d);
    let h_sq = arc_r * arc_r - a * a;
    if h_sq < 0.0 {
        return None;
    }
    let h = h_sq.sqrt();

    let v = (circ_c - arc_c) / d;
    let p = arc_c + a * v;
    // Compute a perpendicular to v. (No special adjustment is needed here.)
    let offset = Vec2::new(-v.y, v.x);

    let inter1 = p + h * offset;
    let inter2 = p - h * offset;

    // Helper to compute the angle (from center arc_c) in our y-down system:
    // we flip the y value when using atan2.
    let angle_from_center =
        |pt: Vec2| -> f64 { ((pt - arc_c).y).atan2((pt - arc_c).x).rem_euclid(2.0 * PI) };

    let a1 = angle_from_center(inter1);
    let a2 = angle_from_center(inter2);

    // Normalize the arc's start angle.
    let start_angle = arc.start_angle.rem_euclid(2.0 * PI);
    // Compute the arc's end angle.
    let end_angle = (arc.start_angle + arc.sweep_angle).rem_euclid(2.0 * PI);

    // Check if a given angle lies on the arc's sweep.
    let angle_in_arc = |angle: f64| -> bool {
        if arc.sweep_angle > 0.0 {
            // For a counterclockwise sweep.
            if start_angle <= end_angle {
                angle >= start_angle && angle <= end_angle
            } else {
                angle >= start_angle || angle <= end_angle
            }
        } else {
            // For a clockwise sweep.
            if start_angle >= end_angle {
                angle <= start_angle && angle >= end_angle
            } else {
                angle <= start_angle || angle >= end_angle
            }
        }
    };

    let mut valid_points = Vec::new();
    if angle_in_arc(a1) {
        valid_points.push(inter1);
    }
    if angle_in_arc(a2) {
        valid_points.push(inter2);
    }

    if valid_points.len() == 1 {
        Some(valid_points[0])
    } else {
        // Either no intersection on the arc, or ambiguous (two points)
        None
    }
}

/// CORRECTED VERSION WITH Y AXIS INVERTED
pub fn arc_from_three_points(p1: Vec2, p2: Vec2, p3: Vec2) -> Option<Arc> {
    // Coordinates of points.
    let (x1, y1) = (p1.x, p1.y);
    let (x2, y2) = (p2.x, p2.y);
    let (x3, y3) = (p3.x, p3.y);
    // Compute determinant to check for collinearity.
    let d = 2.0 * (x1 * (y2 - y3) + x2 * (y3 - y1) + x3 * (y1 - y2));
    if d.abs() < EPSILON {
        return None;
    }
    // Compute circle center (using the standard formula).
    let x_center = ((x1 * x1 + y1 * y1) * (y2 - y3)
        + (x2 * x2 + y2 * y2) * (y3 - y1)
        + (x3 * x3 + y3 * y3) * (y1 - y2))
        / d;
    let y_center = ((x1 * x1 + y1 * y1) * (x3 - x2)
        + (x2 * x2 + y2 * y2) * (x1 - x3)
        + (x3 * x3 + y3 * y3) * (x2 - x1))
        / d;
    let center = Vec2::new(x_center, y_center);
    let radius = (p1 - center).length();
    if radius.abs() < EPSILON {
        return None;
    }
    let angle_from_center = |p: Vec2| ((p - center).y).atan2((p - center).x).rem_euclid(2.0 * PI);
    let a1 = angle_from_center(p1);
    let a2 = angle_from_center(p2);
    let a3 = angle_from_center(p3);
    // We assume the arc should start at p1, pass through p2, and end at p3.
    // There are two arcs connecting p1 and p3. We choose the one that passes through p2.
    let candidate_sweep = (a3 - a1).rem_euclid(2.0 * PI);
    // Check where p2 lies relative to a1.
    let diff2 = (a2 - a1).rem_euclid(2.0 * PI);
    let sweep = if diff2 <= candidate_sweep {
        candidate_sweep
    } else {
        // If p2 is not between a1 and a3 in the positive direction,
        // choose the negative (clockwise) sweep.
        candidate_sweep - 2.0 * PI
    };
    Some(Arc {
        center: center.to_point(),
        // Both radii components are the same, so this is a circle.
        radii: Vec2::new(radius, radius),
        start_angle: a1,
        sweep_angle: sweep,
        x_rotation: 0.0,
    })
}
pub fn circle_from_three_points(p1: Vec2, p2: Vec2, p3: Vec2) -> Option<(Vec2, f64)> {
    // Coordinates of points.
    let (x1, y1) = (p1.x, p1.y);
    let (x2, y2) = (p2.x, p2.y);
    let (x3, y3) = (p3.x, p3.y);
    // Compute determinant to check for collinearity.
    let d = 2.0 * (x1 * (y2 - y3) + x2 * (y3 - y1) + x3 * (y1 - y2));
    if d.abs() < EPSILON {
        return None;
    }
    // Compute circle center using the standard formula.
    let x_center = ((x1 * x1 + y1 * y1) * (y2 - y3)
        + (x2 * x2 + y2 * y2) * (y3 - y1)
        + (x3 * x3 + y3 * y3) * (y1 - y2))
        / d;
    let y_center = ((x1 * x1 + y1 * y1) * (x3 - x2)
        + (x2 * x2 + y2 * y2) * (x1 - x3)
        + (x3 * x3 + y3 * y3) * (x2 - x1))
        / d;
    let center = Vec2::new(x_center, y_center);
    // Compute the radius from the center to any of the points.
    let radius = (p1 - center).length();
    (radius.abs() > EPSILON).then_some((center, radius))
}

pub fn angle0_90(angle: f64) -> f64 {
    let mut angle = angle.rem_euclid(2.0 * PI);
    if angle > PI {
        angle -= 2.0 * PI;
    }
    if angle < 0.0 {
        angle += 2.0 * PI;
    }
    if angle > PI / 2.0 {
        angle = PI - angle;
    }
    if angle < -PI / 2.0 {
        angle = -angle - PI;
    }
    angle.abs()
}
/// CORRECTED VERSION WITH Y AXIS INVERTED ----
pub fn arc_from_center_and_points(center: Vec2, start: Vec2, end: Vec2) -> Option<Arc> {
    // Compute vectors from the center to the start and end points.
    let v_start = start - center;
    let v_end = end - center;

    // Compute the radius from the start point. It should be > 0.
    let radius = v_start.length();
    if radius.abs() < EPSILON {
        // Degenerate: the start point is at the center.
        return None;
    }

    // Ensure that the end point is approximately on the same circle.
    if (v_end.length() - radius).abs() > EPSILON {
        log!(
            "ddddd v_end.length():{:.2}, radius:{:.2}",
            v_end.length(),
            radius
        );
        return None;
    }

    // Compute the angles from the center to the start and end.
    let start_angle = v_start.y.atan2(v_start.x);
    let end_angle = v_end.y.atan2(v_end.x);

    // Normalize the sweep angle to be within (-PI, PI] using rem_euclid.
    let sweep = (end_angle - start_angle + PI).rem_euclid(2.0 * PI) - PI;

    // Degenerate: if the sweep is zero (or nearly zero) there is no meaningful arc.
    if sweep.abs() < EPSILON {
        return None;
    }

    Some(Arc {
        center: center.to_point(),
        radii: Vec2::new(radius, radius),
        start_angle,
        sweep_angle: sweep,
        x_rotation: 0.0,
    })
}

/// CORRECTED VERSION WITH Y AXIS INVERTED
pub fn sub_arc(arc: Arc, pt1: Vec2, pt2: Vec2) -> Option<Arc> {
    let center = arc.center.to_vec2();

    // Compute the vectors from the center to the points.
    let v1 = pt1 - center;
    let v2 = pt2 - center;

    // Invert the y components for a coordinate system where y increases downward.
    let a1 = (-v1.y).atan2(v1.x);
    let a2 = (-v2.y).atan2(v2.x);

    // rem_euclid angles to be within [0, 2π).
    let a1 = a1.rem_euclid(2.0 * PI);
    let a2 = a2.rem_euclid(2.0 * PI);

    // Compute the sweep based on the sign of the original arc's sweep angle.
    let sweep = if arc.sweep_angle > 0. {
        (a2 - a1).rem_euclid(2.0 * PI)
    } else {
        -(a1 - a2).rem_euclid(2.0 * PI)
    };

    Some(Arc {
        center: arc.center,
        radii: arc.radii,
        start_angle: a1,
        sweep_angle: sweep,
        x_rotation: 0.0,
    })
}

/// CORRECTED VERSION WITH Y AXIS INVERTED ----
pub fn arc_start_end_points(arc: &Arc) -> (Vec2, Vec2) {
    let center = arc.center.to_vec2();
    let r = arc.radii.x;
    let start_point = center + Vec2::new(r * arc.start_angle.cos(), r * arc.start_angle.sin());
    let end_angle = arc.start_angle + arc.sweep_angle;
    let end_point = center + Vec2::new(r * end_angle.cos(), r * end_angle.sin());
    (start_point, end_point)
}

/// NO CORRECTION NEEDED FOR Y AXIS INVERTED
pub fn intersect_lines(p1: Vec2, d1: Vec2, p2: Vec2, d2: Vec2) -> Option<Vec2> {
    // Compute the 2D cross product (determinant) of d1 and d2.
    let denom = d1.x * d2.y - d1.y * d2.x;
    // Check if the lines are parallel (or nearly so).
    if denom.abs() < EPSILON {
        return None; // No unique intersection.
    }
    let diff = p2 - p1;
    // Compute the cross product of the difference and d2.
    let t = (diff.x * d2.y - diff.y * d2.x) / denom;
    Some(p1 + d1 * t)
}

pub fn intersect_circles(c1: Vec2, r1: f64, c2: Vec2, r2: f64) -> Option<(Vec2, Vec2)> {
    let d = (c2 - c1).hypot();

    // Check for non-intersecting or coincident circles.
    if d > r1 + r2 || d < (r1 - r2).abs() {
        // No intersection.
        return None;
    }
    if d.abs() < 1e-6 && (r1 - r2).abs() < 1e-6 {
        // Circles are coincident.
        return None;
    }

    // 'a' is the distance from c1 to the line joining the intersections.
    let a = (r1 * r1 - r2 * r2 + d * d) / (2.0 * d);

    // p0 is the point along the line from c1 to c2.
    let p0 = c1 + (c2 - c1) * (a / d);

    // 'h' is the distance from p0 to each intersection point.
    let h = (r1 * r1 - a * a).sqrt();

    // The perpendicular vector (normalized).
    let perp = Vec2::new(-(c2.y - c1.y), c2.x - c1.x) / d;

    // Calculate the intersection points.
    let intersection1 = p0 + perp * h;
    let intersection2 = p0 - perp * h;

    // If h is nearly zero, the circles are tangent (one intersection).
    if h.abs() < EPSILON {
        None
    } else {
        Some((intersection1, intersection2))
    }
}

pub fn bissector(p1: Vec2, apex: Vec2, p3: Vec2) -> Option<(Vec2, f64, Vec2, Vec2)> {
    // Compute the raw vectors from the apex to p1 and p3.
    let v1 = p1 - apex;
    let v2 = p3 - apex;

    // Check that both vectors are non-zero to avoid division by zero during normalization.
    if v1.length() < EPSILON || v2.length() < EPSILON {
        return None;
    }

    // Normalize the vectors.
    let u = v1.normalize();
    let v = v2.normalize();

    // Sum the unit vectors.
    let sum = u + v;

    // Check if the sum is nearly zero (this happens if u and v are opposite directions).
    if sum.length() < EPSILON {
        return None;
    }

    // Normalize the sum to get the bisector direction.
    let bisector = sum.normalize();

    // Compute the angle between u and v, clamping the dot product to the [-1, 1] range to avoid numerical issues.
    let dot = u.dot(v).clamp(-1.0, 1.0);
    let angle = dot.acos();

    if angle.abs() < EPSILON {
        return None;
    }

    // Return the bisector direction and half the angle.
    Some((bisector, angle / 2.0, u, v))
}

/// Returns true if `point` lies within the wedge defined by (p1, apex, p3).
/// The wedge is the area between the two rays starting at `apex` and going through `p1` and `p3`.
pub fn is_point_inside_wedge(p1: Vec2, apex: Vec2, p3: Vec2, point: Vec2) -> bool {
    // Compute vectors from the apex to the boundaries and the point.
    let v1 = p1 - apex;
    let v2 = p3 - apex;
    let vp = point - apex;

    // Compute the cross products.
    // These help to determine the relative orientation of the vectors.
    let cross_wedge = v1.cross(v2);
    let cross1 = v1.cross(vp);
    let cross2 = vp.cross(v2);

    // Depending on the orientation (clockwise or counterclockwise) of the wedge,
    // the point is inside if it lies between the two boundary vectors.
    if cross_wedge >= 0.0 {
        cross1 >= 0.0 && cross2 >= 0.0
    } else {
        cross1 <= 0.0 && cross2 <= 0.0
    }
}

pub fn get_sagitta_pt(v0: Vec2, v1: Vec2, sag_rel: f64) -> Option<Vec2> {
    SegBundle::new(v0, v1).map(|sb| sb.m - sb.n * sb.len * sag_rel)
}

pub fn circle_line_intersection(
    c: Vec2,
    r: f64,
    pt: Vec2,
    u_dir: Vec2,
) -> Option<(Vec2, Option<Vec2>)> {
    // Compute vector from circle center to the line point.
    let d = pt - c;

    // Coefficients for the quadratic equation:
    // A * t^2 + B * t + C = 0, where the line is pt + t*u_dir.
    let a = u_dir.dot(u_dir); // if u_dir is unit, then a == 1.0
    let b = 2.0 * d.dot(u_dir);
    let c_val = d.dot(d) - r * r;

    // Compute the discriminant.
    let disc = b * b - 4.0 * a * c_val;

    // If the discriminant is negative, no real intersections.
    if disc < 0.0 {
        return None;
    }

    // Compute the first intersection.
    let sqrt_disc = disc.sqrt();
    let t1 = (-b + sqrt_disc) / (2.0 * a);
    let p1 = pt + u_dir * t1;

    // If the discriminant is nearly zero, there's one intersection.
    if disc.abs() < 1e-10 {
        return Some((p1, None));
    }

    // Otherwise, compute the second intersection.
    let t2 = (-b - sqrt_disc) / (2.0 * a);
    let p2 = pt + u_dir * t2;

    Some((p1, Some(p2)))
}

/// Projects a point `p` onto a line defined by `line_point` and a unit direction `u_dir`.
pub fn project_point_on_line(p: Vec2, line_point: Vec2, line_u_dir: Vec2) -> Vec2 {
    // Vector from the line point to the given point.
    let v = p - line_point;

    // Compute the projection length along the line (using the unit direction).
    let proj_length = v.dot(line_u_dir);

    // Compute the projection point.
    line_point + line_u_dir * proj_length
}
/// Returns the nearest intersection point on the circle circumference with
/// the line passing through the given point and the circle's center.
/// If the point is exactly at the center, returns None.
pub fn nearest_circle_point(center: Vec2, radius: f64, p: Vec2) -> Option<Vec2> {
    let diff = p - center;
    let d = diff.hypot();

    // Check that the point is not exactly at the center.
    if d < EPSILON {
        return None;
    }

    // Compute the unit direction from center to the point.
    let unit_dir = diff / d;

    // The intersection point on the circumference.
    Some(center + unit_dir * radius)
}

pub fn get_dad(v: Vec2, apex: Vec2, w: Vec2) -> Option<(f64, f64, f64)> {
    let v = v - apex;
    let w = w - apex;
    let dv = v.hypot();
    let dw = w.hypot();

    // Early out if any leg is ~zero length
    if dv <= EPSILON || dw <= EPSILON {
        return None;
    }

    Some((dv, f64::atan2(v.cross(w), v.dot(w)), dw))
}

pub fn fillet_at_apex(a: Vec2, b: Vec2, c: Vec2, r: f64) -> Option<(Vec2, Vec2, Vec2)> {
    // vectors from apex
    let ba = a - b;
    let bc = c - b;

    // lengths (reject degenerate)
    let la2 = ba.x * ba.x + ba.y * ba.y;
    let lc2 = bc.x * bc.x + bc.y * bc.y;
    if la2 == 0.0 || lc2 == 0.0 || r <= 0.0 {
        return None;
    }
    let la = la2.sqrt();
    let lc = lc2.sqrt();

    // unit directions from apex toward a and c
    let u = ba / la;
    let v = bc / lc;

    // interior angle at apex (unsigned) using atan2 for robustness
    let theta = f64::atan2(u.cross(v), u.dot(v)).abs(); // in (0, π]

    // No fillet if angle is ~0 or ~π (colinear)
    let half = 0.5 * theta;
    let s_half = half.sin();
    let t_half = half.tan();
    if s_half <= EPSILON || t_half <= EPSILON {
        return None;
    }

    // trim distance along each edge
    let t = r / t_half; // r * cot(theta/2)
    if t >= la || t >= lc {
        // fillet would overshoot the segments
        return None;
    }

    // arc endpoints on the edges
    let s = b + u * t; // along BA
    let e = b + v * t; // along BC

    // center along the angle bisector
    // bisector direction is normalized (u + v). If u ≈ -v (theta≈π), this is near zero (handled above).
    let bis = u + v;
    let bis_len = (bis.x * bis.x + bis.y * bis.y).sqrt();
    if bis_len <= EPSILON {
        return None;
    }
    let bis_dir = bis / bis_len;

    let d = r / s_half; // distance from apex to center
    let center = b + bis_dir * d;

    Some((s, center, e))
}

#[derive(Copy, Debug, Clone)]
pub enum ApexType {
    TypeVertex { a: Vec2 },
    Arc { s: Vec2, c: Vec2, e: Vec2 }, // c = arc center
}

fn to_point(v: Vec2) -> Point {
    Point::new(v.x, v.y)
}

// helper: atan2(y, x)
fn angle(p: Vec2) -> f64 {
    p.y.atan2(p.x)
}

fn normalize_sweep(mut d: f64) -> f64 {
    // bring to (-PI, PI]
    while d <= -PI {
        d += 2.0 * PI;
    }
    while d > PI {
        d -= 2.0 * PI;
    }
    d
}

fn add_circular_arc(path: &mut BezPath, center: Vec2, from: Vec2, to: Vec2) {
    // vectors relative to center
    let p0 = from - center;
    let p1 = to - center;

    let r0 = (p0.x * p0.x + p0.y * p0.y).sqrt();
    let r1 = (p1.x * p1.x + p1.y * p1.y).sqrt();
    if r0 == 0.0 || r1 == 0.0 {
        return;
    }

    // use average radius in case of tiny numeric mismatch
    let r = 0.5 * (r0 + r1);

    let a0 = angle(p0);
    let a1 = angle(p1);
    let sweep = normalize_sweep(a1 - a0); // signed, shortest arc

    // Ensure path current point is at `from` (caller may already have moved there)
    // path.line_to(KPoint::new(from.x, from.y)); // uncomment if needed

    let arc = Arc {
        center: Point::new(center.x, center.y),
        radii: Vec2::new(r, r),
        start_angle: a0,
        sweep_angle: sweep,
        x_rotation: 0.0,
    };
    arc.to_cubic_beziers(0.01, |p1, p2, p3| {
        path.push(PathEl::CurveTo(p1, p2, p3));
    });
}

pub fn bezpath_from_apices(apices: &[ApexType]) -> BezPath {
    let start_of = |apex: &ApexType| -> Vec2 {
        match *apex {
            ApexType::TypeVertex { a } => a,
            ApexType::Arc { s, .. } => s,
        }
    };

    // Starting point = entry of apex 0
    let mut path = BezPath::new();
    let start = start_of(&apices[0]);
    path.move_to(to_point(start));
    let mut curr = start;

    let n = apices.len();
    for i in 0..n {
        let this = &apices[i];
        let next_entry = start_of(&apices[(i + 1) % n]);

        match *this {
            ApexType::TypeVertex { a } => {
                // We should be at 'a'. If not (only possible at i==0 when previous was arc),
                // first line to 'a'.
                if curr != a {
                    path.line_to(to_point(a));
                    curr = a;
                }
                // Then line to next entry
                if curr != next_entry {
                    path.line_to(to_point(next_entry));
                    curr = next_entry;
                }
            }
            ApexType::Arc { s, c, e } => {
                // Ensure we're at the arc start s
                if curr != s {
                    path.line_to(to_point(s));
                    // curr = s;
                }
                // Append the circular arc s -> e with center c
                add_circular_arc(&mut path, c, s, e);
                curr = e;

                // Then line to the next entry if needed
                if curr != next_entry {
                    path.line_to(to_point(next_entry));
                    curr = next_entry;
                }
            }
        }
    }

    path.close_path();
    path
}
