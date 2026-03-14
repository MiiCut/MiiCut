use crate::{
    canvas::{CanvasText, CanvasTextConfig, Colors, Pattern, TextAlign, TextPos},
    helpers::prefab::*,
    types::others::SegBundle,
};
use kurbo::{BezPath, Rect, Vec2};
use std::{f64::consts::PI, vec};

/// Classic CAD-style dimension annotation (extension lines + dimension line + arrowheads + handle).
/// p1, p2: edge endpoints; offset: perpendicular offset from edge to dim line; label: text.
/// Returns (path, pattern, colors, texts, handle_pos).
pub fn dim_classic(
    p1: Vec2,
    p2: Vec2,
    offset: f64,
    label: String,
    cinfo: (Rect, f64, Vec2),
) -> (BezPath, Pattern, Colors, Vec<CanvasText>, Vec2) {
    let scale = cinfo.1;
    let color = get_dimension_colors();
    let text_color = get_text_colors().stroke_color;

    let (path, handle) = dim_classic_path(p1, p2, offset, scale);

    let edge = p2 - p1;
    let len = edge.hypot();
    let raw_angle = if len > 0.1 { edge.y.atan2(edge.x) } else { 0.0 };
    let angle = if raw_angle > -PI / 2.0 && raw_angle <= PI / 2.0 {
        raw_angle
    } else {
        raw_angle - PI
    };

    // Direction "au-dessus de la ligne de base" du texte dans le canvas y-down : (sin θ, -cos θ)
    let above_baseline = Vec2::new(angle.sin(), -angle.cos());
    let text_pos = handle + above_baseline * (6.0 / scale);

    let text = CanvasText::new(
        label,
        TextPos::PosCustom(text_pos),
        CanvasTextConfig::new(text_color, angle, TextAlign::Center, 14, 0.8),
    );

    (path, Pattern::Dim, color, vec![text], handle)
}

/// Radius dimension for a Disc shape with rotatable handle.
/// center: circle center, radius: circle radius, angle: display angle (radians).
/// Returns (path, pattern, colors, texts, handle_pos).
pub fn dim_disc(
    center: Vec2,
    radius: f64,
    angle: f64,
    cinfo: (Rect, f64, Vec2),
) -> (BezPath, Pattern, Colors, Vec<CanvasText>, Vec2) {
    let scale = cinfo.1;
    let color = get_dimension_colors();
    let text_color = get_text_colors().stroke_color;

    let (path, handle) = dim_radius_classic(center, radius, angle, scale);

    let u = Vec2::new(angle.cos(), angle.sin());
    let _n = Vec2::new(-u.y, u.x);

    // Flip angle to keep text readable (never upside-down)
    let flipped = angle.cos() < 0.0;
    let display_angle = if flipped {
        angle + std::f64::consts::PI
    } else {
        angle
    };

    // Direction "au-dessus de la ligne de base" pour le canvas y-down : (sin θ, -cos θ)
    let above_baseline = Vec2::new(display_angle.sin(), -display_angle.cos());
    let text_pos = center + u * (radius * 0.55) + above_baseline * (6.0 / scale);

    let text = CanvasText::new(
        format!("{:.1}", radius),
        TextPos::PosCustom(text_pos),
        CanvasTextConfig::new(text_color, display_angle, TextAlign::Center, 14, 0.8),
    );

    (path, Pattern::Dim, color, vec![text], handle)
}

#[allow(dead_code)]
const HU: Vec2 = Vec2::new(1., 0.);
#[allow(dead_code)]
const VU: Vec2 = Vec2::new(0., 1.);

const EPSILON_DEG: f64 = 1.;

pub fn dim_hv(
    bdl: SegBundle,
    cinfo: (Rect, f64, Vec2),
    arrow: bool,
) -> (BezPath, Pattern, Colors, Vec<CanvasText>) {
    let color = get_dimension_colors();
    let dim_offset = 15. / cinfo.1;
    let mut texts = vec![];
    let (angle, txt_off) = if bdl.a > -PI / 2. && bdl.a < PI / 2. {
        (bdl.a, -1.)
    } else {
        (bdl.a - PI, 1.)
    };
    match (
        (bdl.s.x - bdl.e.x).abs() >= 0.1,
        (bdl.s.y - bdl.e.y).abs() >= 0.1,
    ) {
        (true, true) => {
            // Horizontal dimension
            texts.push(CanvasText::new(
                format!("H{:4.0}", (bdl.e.x - bdl.s.x).abs()),
                TextPos::PosCustom(bdl.m + bdl.n * txt_off * 0.5 * (dim_offset)),
                CanvasTextConfig::new(
                    get_text_colors().stroke_color,
                    angle,
                    TextAlign::Center,
                    14,
                    0.8,
                ),
            ));
            // Vertical dimension
            texts.push(CanvasText::new(
                format!("V{:4.0}", (bdl.e.y - bdl.s.y).abs()),
                TextPos::PosCustom(bdl.m - bdl.n * txt_off.abs() * (txt_off * dim_offset)),
                CanvasTextConfig::new(
                    get_text_colors().stroke_color,
                    angle,
                    TextAlign::Center,
                    14,
                    0.8,
                ),
            ));
        }
        (true, false) => {
            // Horizontal dimension
            texts.push(CanvasText::new(
                format!("H{:4.0}", (bdl.e.x - bdl.s.x).abs()),
                TextPos::PosCustom(bdl.m + bdl.n * txt_off * 0.5 * (dim_offset)),
                CanvasTextConfig::new(
                    get_text_colors().stroke_color,
                    angle,
                    TextAlign::Center,
                    14,
                    0.8,
                ),
            ));
        }
        (false, true) => {
            // Vertical dimension
            texts.push(CanvasText::new(
                format!("V{:4.0}", (bdl.e.y - bdl.s.y).abs()),
                TextPos::PosCustom(bdl.m - bdl.n * txt_off.abs() * (txt_off * dim_offset)),
                CanvasTextConfig::new(
                    get_text_colors().stroke_color,
                    angle,
                    TextAlign::Center,
                    14,
                    0.8,
                ),
            ));
        }
        (false, false) => (),
    }

    let path = if arrow {
        dim_arrow_path(&bdl, cinfo.1)
    } else {
        dim_path(&bdl, cinfo.1)
    };
    (path, Pattern::Dim, color, texts)
}

pub fn dim_radius(
    bdl: SegBundle,
    cinfo: (Rect, f64, Vec2),
    want_radius: bool,
    want_angle: bool,
) -> (BezPath, Pattern, Colors, Vec<CanvasText>) {
    use Pattern::*;

    let color = get_dimension_colors();
    let dim_offset = 10.0 / cinfo.1;

    let (angle, txt_off) = if (-PI / 2.0..PI / 2.0).contains(&bdl.a) {
        (bdl.a, -1.0)
    } else {
        (bdl.a - PI, 1.0)
    };

    // Shared text style
    let text_color = get_text_colors().stroke_color;
    let text_cfg = CanvasTextConfig::new(text_color, angle, TextAlign::Center, 14, 0.8);

    // Label with optional "R" prefix
    let prefix = if want_radius { "R" } else { "" };
    let text1 = CanvasText::new(
        format!("{prefix}{:.1}", bdl.len),
        TextPos::PosCustom(bdl.m - 2. * bdl.n),
        text_cfg.clone(),
    );

    let mut path = BezPath::new();
    path.move_to(bdl.s.to_point());
    path.line_to(bdl.e.to_point());

    // Angle label only if not ~0, ~90, or ~180 degrees
    let deg = bdl.a * 180.0 / PI;
    let d = (deg.rem_euclid(180.0)).abs(); // fold 180° to 0°
    let near = |x: f64| (d - x).abs() <= EPSILON_DEG;

    if want_angle {
        if !(near(0.0) || near(90.0)) {
            let text2 = CanvasText::new(
                format!("α{:.1}°", deg),
                TextPos::PosCustom(bdl.m - bdl.n * (txt_off * dim_offset)),
                text_cfg,
            );
            (path, Dim, color, vec![text1, text2])
        } else {
            (path, Dim, color, vec![text1])
        }
    } else {
        // If angle is not needed, return only the path and first text
        (path, Dim, color, vec![text1])
    }
}
