use kurbo::{BezPath, Circle, PathEl, Shape, Vec2};

use crate::{
    canvas::{Color, Colors},
    pools::HS,
    positions::Status,
};

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
    let size = 5.;
    Circle::new(pos.to_point(), size / scale).to_path(tol)
}
pub fn line_path(pos1: Vec2, pos2: Vec2) -> BezPath {
    BezPath::from_vec(vec![
        PathEl::MoveTo(pos1.to_point()),
        PathEl::LineTo(pos2.to_point()),
    ])
}

pub fn get_helpers_colors(state: Status) -> Colors {
    use HS::*;
    match (state.is_hs(Select), state.is_hs(Highlight)) {
        (true, _) => Colors {
            color: Color::Gray,
            fill_color: Color::Gray,
        },
        (false, false) => Colors {
            color: Color::Gray,
            fill_color: Color::Gray,
        },
        (false, true) => Colors {
            color: Color::Gray,
            fill_color: Color::Gray,
        },
    }
}

pub fn get_shapes_colors(state: Status) -> Colors {
    use HS::*;
    match (state.is_hs(Select), state.is_hs(Highlight)) {
        (true, _) => Colors {
            color: Color::Gray,
            fill_color: Color::Gray,
        },
        (false, false) => Colors {
            color: Color::Gray,
            fill_color: Color::Gray,
        },
        (false, true) => Colors {
            color: Color::Gray,
            fill_color: Color::Gray,
        },
    }
}

pub fn get_shapes_point_colors(state: Status) -> Colors {
    use HS::*;
    match (state.is_hs(Select), state.is_hs(Highlight)) {
        (true, _) => Colors {
            color: Color::Gray,
            fill_color: Color::Gray,
        },
        (false, false) => Colors {
            color: Color::Gray,
            fill_color: Color::Gray,
        },
        (false, true) => Colors {
            color: Color::Gray,
            fill_color: Color::Gray,
        },
    }
}

pub fn get_shapes_centroid_colors(state: Status) -> Colors {
    use HS::*;
    match (state.is_hs(Select), state.is_hs(Highlight)) {
        (true, _) => Colors {
            color: Color::Gray,
            fill_color: Color::Gray,
        },
        (false, false) => Colors {
            color: Color::Gray,
            fill_color: Color::Gray,
        },
        (false, true) => Colors {
            color: Color::Gray,
            fill_color: Color::Gray,
        },
    }
}

pub fn get_dim_colors(state: Status) -> Colors {
    use HS::*;
    match (state.is_hs(Select), state.is_hs(Highlight)) {
        (true, _) => Colors {
            color: Color::Gray,
            fill_color: Color::Gray,
        },
        (false, false) => Colors {
            color: Color::Gray,
            fill_color: Color::Gray,
        },
        (false, true) => Colors {
            color: Color::Gray,
            fill_color: Color::Gray,
        },
    }
}
