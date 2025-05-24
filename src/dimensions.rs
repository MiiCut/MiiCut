use crate::{
    canvas::{CanvasText, CanvasTextConfig, Color, Colors, Pattern, TextAlign, TextPos},
    math::*,
    types::SegBundle,
};
use kurbo::{BezPath, Rect, Vec2};
use std::f64::consts::PI;

pub fn dim_linear(
    bdl: SegBundle,
    cinfo: (Rect, f64, Vec2),
) -> (BezPath, Pattern, Colors, Vec<CanvasText>) {
    use Pattern::*;

    let color = Colors {
        color: Color::Black,
        fill_color: Color::Black,
    };

    let dim_offset = 15. / cinfo.1;
    let mut path = BezPath::new();
    let (angle, txt_off) = if bdl.a > -PI / 2. && bdl.a < PI / 2. {
        (bdl.a, 1.5)
    } else {
        (bdl.a - PI, 2.5)
    };
    let text = CanvasText::new(
        format!("{:.1}", bdl.len),
        TextPos::PosCustom(bdl.m - bdl.n * (txt_off * dim_offset)),
        CanvasTextConfig::new(color.color, angle, TextAlign::Center, 14, 0.8),
    );
    let s = bdl.s - bdl.n * dim_offset;
    let e = bdl.e - bdl.n * dim_offset;
    arrow(&mut path, s, bdl.u, dim_offset);
    line(&mut path, s, e);
    arrow(&mut path, e, -bdl.u, dim_offset);
    (path, Dim, color, vec![text])
}

pub fn dim_linear_angle(
    bdl: SegBundle,
    cinfo: (Rect, f64, Vec2),
) -> (BezPath, Pattern, Colors, Vec<CanvasText>) {
    use Pattern::*;

    let color = Colors {
        color: Color::Black,
        fill_color: Color::Black,
    };

    let dim_offset = 15. / cinfo.1;
    let mut path = BezPath::new();
    let (angle, txt_off) = if bdl.a >= -PI / 2. && bdl.a < PI / 2. {
        (bdl.a, 1.5)
    } else {
        (bdl.a - PI, 2.5)
    };
    let angle_clip90 = angle0_90(bdl.a);
    let text = if angle_clip90 < EPSILON || (angle_clip90 - PI / 2.).abs() < EPSILON {
        format!("{:.1}mm", bdl.len)
    } else {
        format!("{:.0}° - {:.1}mm", angle_clip90 / PI * 180., bdl.len)
    };

    let canvas_text = CanvasText::new(
        text,
        TextPos::PosCustom(bdl.m - bdl.n * (txt_off * dim_offset)),
        CanvasTextConfig::new(color.color, angle, TextAlign::Center, 14, 0.8),
    );
    let s = bdl.s - bdl.n * dim_offset;
    let e = bdl.e - bdl.n * dim_offset;
    arrow(&mut path, s, bdl.u, dim_offset);
    line(&mut path, s, e);
    arrow(&mut path, e, -bdl.u, dim_offset);
    (path, Dim, color, vec![canvas_text])
}

pub fn dim_radius(
    bdl: SegBundle,
    cinfo: (Rect, f64, Vec2),
) -> (BezPath, Pattern, Colors, Vec<CanvasText>) {
    use Pattern::*;

    let color = Colors {
        color: Color::Black,
        fill_color: Color::Black,
    };

    let dim_offset = 10. / cinfo.1;
    let mut path = BezPath::new();
    let (angle, txt_off) = if bdl.a > -PI / 2. && bdl.a < PI / 2. {
        (bdl.a, 1.5)
    } else {
        (bdl.a - PI, 2.5)
    };
    let text = CanvasText::new(
        format!("R: {:.1}", bdl.len),
        TextPos::PosCustom(bdl.m - bdl.n * (txt_off * dim_offset)),
        CanvasTextConfig::new(color.color, angle, TextAlign::Center, 14, 0.8),
    );
    let s = bdl.s; // - bdl.n * dim_offset;
    let e = bdl.e; // - bdl.n * dim_offset;
    arrow(&mut path, s, bdl.u, dim_offset);
    line(&mut path, s, e);
    arrow(&mut path, e, -bdl.u, dim_offset);
    (path, Dim, color, vec![text])
}

fn line(path: &mut BezPath, pt1: Vec2, pt2: Vec2) {
    path.move_to(pt1.to_point());
    path.line_to(pt2.to_point());
}
fn arrow(path: &mut BezPath, pt: Vec2, u: Vec2, dim_offset: f64) {
    let pt1 = pt + (u + 0.4 * Vec2::new(-u.y, u.x)) * dim_offset;
    let pt2 = pt + (u - 0.4 * Vec2::new(-u.y, u.x)) * dim_offset;
    path.move_to(pt.to_point());
    path.line_to(pt1.to_point());
    path.move_to(pt.to_point());
    path.line_to(pt2.to_point());
}
// pub fn get_text_pos(&self) -> TextPos {
//     let center = self.get_center();
//     let angle = self.get_angle();
//     let length = self.get_length();
//     let text_pos = center + Vec2::from_angle(angle) * length / 2.;
//     TextPos::PosCustom(text_pos)
// }

// pub fn get_text_align(&self) -> TextAlign {
//     let angle = self.get_angle();
//     if angle > std::f64::consts::PI / 2. && angle < 3. * std::f64::consts::PI / 2. {
//         TextAlign::Right
//     } else {
//         TextAlign::Left
//     }
// }
// pub fn get_text_val(&self) -> String {
//     format!("{:.2}", self.get_length())
// }
// pub fn get_text(&self) -> CanvasText {
//     CanvasText::new(
//         self.get_text_val(),
//         self.get_text_pos(),
//         CanvasTextConfig::new(
//             self.get_text_pattern(self.selected, self.highlighted),
//             self.get_text_angle(),
//             self.get_text_align(),
//             14,
//             1.,
//         ),
//     )
// }

// fn get_radius_path(&self) -> (BezPath, Pattern, CanvasText) {
//     let mut path = kurbo::BezPath::new();
//     let end = self.end + (self.end - self.start).normalize() * 10.;

//     let text = CanvasText::new(
//         format!("r: {:.2}", value),
//         TextPos::PosCustom(Vec2::new(end.x + 2., end.y - 2.)),
//         CanvasTextConfig::new(
//             self.get_text_pattern(false, false),
//             0.,
//             TextAlign::Left,
//             14,
//             0.5,
//         ),
//     );

//     path.move_to(self.start.to_point());
//     path.line_to(end.to_point());
//     path.line_to(Vec2::new(end.x + 10., end.y).to_point());

//     (path, Pattern::DimensionNormal, text)
// }
// fn get_angle_path(&self) -> (BezPath, Pattern, CanvasText) {
//     let mut path = kurbo::BezPath::new();
//     let start = self.start;
//     let end = self.end;
//     let unit_vec = (end - start).normalize();
//     let ratio = self.get_length() * 0.4;
//     let angle_pt = start + unit_vec * ratio;
//     let angle = (self.end - self.start).atan2();
//     let text_pos_angle = get_point_at_dist_from_angle(start, angle / 2., 10.);

//     let text = CanvasText::new(
//         format!("a: {:.1}", -angle / PI * 180.),
//         TextPos::PosCustom(text_pos_angle),
//         CanvasTextConfig::new(
//             self.get_text_pattern(false, false),
//             0., //self.get_text_angle(),
//             TextAlign::Left,
//             14,
//             0.5,
//         ),
//     );

//     path.move_to((start + Vec2::new(ratio, 0.)).to_point());
//     path.line_to(start.to_point());
//     path.line_to(angle_pt.to_point());

//     (path, Pattern::DimensionNormal, text)
// }
