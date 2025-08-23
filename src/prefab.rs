use kurbo::{BezPath, Circle, PathEl, Shape, Vec2};

use crate::canvas::{Color, Colors};

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
    let size = 5.;
    Circle::new(pos.to_point(), size / scale).to_path(tol)
}
pub fn line_path(pos1: Vec2, pos2: Vec2) -> BezPath {
    BezPath::from_vec(vec![
        PathEl::MoveTo(pos1.to_point()),
        PathEl::LineTo(pos2.to_point()),
    ])
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

// pub fn get_final_contour_colors(state: Status) -> Colors {
//     use HS::*;
//     match (state.is_hs(Select), state.is_hs(Highlight)) {
//         (true, _) => Colors {
//             color: Color::Black,
//             fill_color: Color::Gray90Opacity,
//         },
//         (false, false) => Colors {
//             color: Color::Black,
//             fill_color: Color::Gray90Opacity,
//         },
//         (false, true) => Colors {
//             color: Color::Black,
//             fill_color: Color::Gray90Opacity,
//         },
//     }
// }
// pub fn get_on_creation_colors(state: Status) -> Colors {
//     use HS::*;
//     match (state.is_hs(Select), state.is_hs(Highlight)) {
//         (true, _) => Colors {
//             color: Color::Black65Opacity,
//             fill_color: Color::Gray60Opacity,
//         },
//         (false, false) => Colors {
//             color: Color::Black65Opacity,
//             fill_color: Color::Gray60Opacity,
//         },
//         (false, true) => Colors {
//             color: Color::Black65Opacity,
//             fill_color: Color::Gray60Opacity,
//         },
//     }
// }
// pub fn get_prims_colors(state: Status) -> Colors {
//     use HS::*;
//     match (state.is_hs(Select), state.is_hs(Highlight)) {
//         (true, _) => Colors {
//             color: Color::Purple55Opacity,
//             fill_color: Color::Purple55Opacity,
//         },
//         (false, false) => Colors {
//             color: Color::Purple55Opacity,
//             fill_color: Color::Purple55Opacity,
//         },
//         (false, true) => Colors {
//             color: Color::Purple55Opacity,
//             fill_color: Color::Purple55Opacity,
//         },
//     }
// }

// pub fn get_centroids_colors(state: Status) -> Colors {
//     use HS::*;
//     match (state.is_hs(Select), state.is_hs(Highlight)) {
//         (false, false) => Colors {
//             color: Color::Black65Opacity,
//             fill_color: Color::Transparent,
//         },
//         (false, true) => Colors {
//             color: Color::Black65Opacity,
//             fill_color: Color::Black65Opacity,
//         },
//         (true, _) => Colors {
//             color: Color::Red,
//             fill_color: Color::Red,
//         },
//     }
// }
// pub fn get_helpers_centroid_colors(state: Status) -> Colors {
//     use HS::*;
//     match (state.is_hs(Select), state.is_hs(Highlight)) {
//         (true, _) => Colors {
//             color: Color::Gray,
//             fill_color: Color::Gray,
//         },
//         (false, false) => Colors {
//             color: Color::Gray,
//             fill_color: Color::Gray,
//         },
//         (false, true) => Colors {
//             color: Color::Gray,
//             fill_color: Color::Gray,
//         },
//     }
// }
// pub fn get_dim_colors(state: Status) -> Colors {
//     use HS::*;
//     match (state.is_hs(Select), state.is_hs(Highlight)) {
//         (true, _) => Colors {
//             color: Color::Black65Opacity,
//             fill_color: Color::Black65Opacity,
//         },
//         (false, false) => Colors {
//             color: Color::Black65Opacity,
//             fill_color: Color::Black65Opacity,
//         },
//         (false, true) => Colors {
//             color: Color::Black65Opacity,
//             fill_color: Color::Black65Opacity,
//         },
//     }
// }
// pub fn get_dim_text_colors(state: Status) -> Colors {
//     use HS::*;
//     match (state.is_hs(Select), state.is_hs(Highlight)) {
//         (true, _) => Colors {
//             color: Color::Black65Opacity,
//             fill_color: Color::Black65Opacity,
//         },
//         (false, false) => Colors {
//             color: Color::Black65Opacity,
//             fill_color: Color::Black65Opacity,
//         },
//         (false, true) => Colors {
//             color: Color::Black65Opacity,
//             fill_color: Color::Black65Opacity,
//         },
//     }
// }
