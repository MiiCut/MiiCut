use kurbo::{Arc, BezPath, Circle, PathEl, Point, Shape, Vec2};

use crate::canvas::{Color, Colors};
use crate::types::others::SegBundle;

pub const VERTEX_RADIUS: f64 = 5.0;

pub fn centroid_path(pos: Vec2, _scale: f64, size: f64) -> BezPath {
    use PathEl::*;
    let v: Vec<PathEl> = vec![
        MoveTo(pos.to_point() - (0., size)),
        LineTo(pos.to_point() + (0., size)),
        MoveTo(pos.to_point() - (size, 0.)),
        LineTo(pos.to_point() + (size, 0.)),
    ];
    BezPath::from_vec(v)
}
pub fn helper_point_path(pos: Vec2, size: f64) -> BezPath {
    let tol = 0.01;
    use PathEl::*;
    let mut v: Vec<PathEl> = vec![
        MoveTo(pos.to_point() - (0., 2. * size)),
        LineTo(pos.to_point() + (0., 2. * size)),
        MoveTo(pos.to_point() - (2. * size, 0.)),
        LineTo(pos.to_point() + (2. * size, 0.)),
    ];
    v.extend(Circle::new(pos.to_point(), size).to_path(tol).to_path(tol));
    BezPath::from_vec(v)
}
pub fn point_path(pos: Vec2, scale: f64) -> BezPath {
    let tol = 0.01;
    Circle::new(pos.to_point(), VERTEX_RADIUS / scale).to_path(tol)
}
pub fn line_path(pos1: Vec2, pos2: Vec2) -> BezPath {
    BezPath::from_vec(vec![
        PathEl::MoveTo(pos1.to_point()),
        PathEl::LineTo(pos2.to_point()),
    ])
}

pub fn toolpath_points_to_path(points: &[Vec2]) -> Option<BezPath> {
    if points.len() < 2 {
        return None;
    }
    let mut path = BezPath::new();
    path.push(PathEl::MoveTo(points[0].to_point()));
    for pt in points.iter().skip(1) {
        path.push(PathEl::LineTo(pt.to_point()));
    }
    path.push(PathEl::ClosePath);
    Some(path)
}

pub fn toolpath_arc_path(center: Vec2, start: Vec2, end: Vec2, ccw: bool) -> Option<BezPath> {
    let v0 = start - center;
    let v1 = end - center;
    let r0 = v0.hypot();
    let r1 = v1.hypot();
    if r0 == 0.0 || r1 == 0.0 {
        return None;
    }
    let r = 0.5 * (r0 + r1);
    let a0 = v0.y.atan2(v0.x);
    let a1 = v1.y.atan2(v1.x);
    let mut sweep = a1 - a0;
    if ccw {
        while sweep <= 0.0 {
            sweep += std::f64::consts::PI * 2.0;
        }
    } else {
        while sweep >= 0.0 {
            sweep -= std::f64::consts::PI * 2.0;
        }
    }

    let mut path = BezPath::new();
    path.push(PathEl::MoveTo(start.to_point()));
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
    Some(path)
}

pub fn toolpath_arrowhead_path(start: Vec2, dir: Vec2, size: f64) -> Option<BezPath> {
    if dir.hypot() <= 0.0001 {
        return None;
    }
    let dir = dir.normalize();
    let perp = Vec2::new(-dir.y, dir.x);
    let tip = start + dir * size;
    let left = start + perp * (size * 0.5);
    let right = start - perp * (size * 0.5);

    let mut path = BezPath::new();
    path.push(PathEl::MoveTo(tip.to_point()));
    path.push(PathEl::LineTo(left.to_point()));
    path.push(PathEl::LineTo(right.to_point()));
    path.push(PathEl::ClosePath);
    Some(path)
}

pub fn dim_arrow_path(seg: &SegBundle, scale: f64) -> BezPath {
    let mut path = BezPath::new();
    path.move_to(seg.s.to_point());
    path.line_to(seg.e.to_point());
    let arrow_len = 20.0 / scale;
    let arrow_w = 6.0 / scale;
    let tip = seg.s + (seg.e - seg.s) * 0.55;
    let base = tip - seg.u * arrow_len;
    let left = base + seg.n * arrow_w;
    let right = base - seg.n * arrow_w;
    path.move_to(tip.to_point());
    path.line_to(left.to_point());
    path.line_to(right.to_point());
    path.close_path();
    path
}
pub fn dim_path(seg: &SegBundle, _scale: f64) -> BezPath {
    let mut path = BezPath::new();
    path.move_to(seg.s.to_point());
    path.line_to(seg.e.to_point());
    path
}

pub fn get_stroke_color(selected: bool, highlighted: bool) -> Color {
    match (selected, highlighted) {
        (false, false) => Color::Gray,
        (false, true) => Color::Green40,
        (true, false) => Color::Red,
        (true, true) => Color::Red,
    }
}
pub fn get_fill_color(selected: bool, highlighted: bool) -> Color {
    match (selected, highlighted) {
        (false, false) => Color::Transparent,
        (false, true) => Color::Green40,
        (true, false) => Color::Red30,
        (true, true) => Color::Red30,
    }
}

pub fn get_vertices_colors(selected: bool, highlighted: bool) -> Colors {
    match (selected, highlighted) {
        (false, false) => Colors {
            stroke_color: Color::Black,
            fill_color: Color::Gray95,
        },
        (false, true) => Colors {
            stroke_color: Color::Black,
            fill_color: Color::Green40,
        },
        (true, false) => Colors {
            stroke_color: Color::Black,
            fill_color: Color::Red30,
        },
        (true, true) => Colors {
            stroke_color: Color::Black,
            fill_color: Color::Red60,
        },
    }
}

pub fn get_dimension_colors() -> Colors {
    Colors {
        stroke_color: Color::Gray20,
        fill_color: Color::Gray,
    }
}

pub fn get_text_colors() -> Colors {
    Colors {
        stroke_color: Color::Gray,
        fill_color: Color::Olive60,
    }
}
