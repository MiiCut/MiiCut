#[derive(Clone, Copy, Debug)]
pub struct Seg {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub cut: bool,
} // cut=true => torche ON

pub fn gcode_to_segments(gcode: &str) -> Vec<Seg> {
    let mut segs = Vec::new();
    let mut x = 0.0;
    let mut y = 0.0;
    let mut torch_on = false;

    for raw in gcode.lines() {
        let line = raw.split(';').next().unwrap_or("").trim(); // retire commentaires ';'
        if line.is_empty() {
            continue;
        }

        let up = line.to_ascii_uppercase();

        if up.starts_with("M3") {
            torch_on = true;
            continue;
        }
        if up.starts_with("M5") {
            torch_on = false;
            continue;
        }

        let is_g0 = up.starts_with("G0") || up.starts_with("G00");
        let is_g1 = up.starts_with("G1") || up.starts_with("G01");
        let is_g2 = up.starts_with("G2") || up.starts_with("G02");
        let is_g3 = up.starts_with("G3") || up.starts_with("G03");
        if !is_g0 && !is_g1 && !is_g2 && !is_g3 {
            continue;
        }

        let mut nx = None::<f64>;
        let mut ny = None::<f64>;
        let mut ni = None::<f64>;
        let mut nj = None::<f64>;

        for tok in up.split_whitespace() {
            if let Some(v) = tok.strip_prefix('X') {
                if let Ok(f) = v.parse::<f64>() {
                    nx = Some(f);
                }
            } else if let Some(v) = tok.strip_prefix('Y') {
                if let Ok(f) = v.parse::<f64>() {
                    ny = Some(f);
                }
            } else if let Some(v) = tok.strip_prefix('I') {
                if let Ok(f) = v.parse::<f64>() {
                    ni = Some(f);
                }
            } else if let Some(v) = tok.strip_prefix('J') {
                if let Ok(f) = v.parse::<f64>() {
                    nj = Some(f);
                }
            }
        }

        let x2 = nx.unwrap_or(x);
        let y2 = ny.unwrap_or(y);

        if (is_g2 || is_g3) && (ni.is_some() || nj.is_some()) && (x2 != x || y2 != y) {
            let i = ni.unwrap_or(0.0);
            let j = nj.unwrap_or(0.0);
            let cx = x + i;
            let cy = y + j;
            let start_angle = (y - cy).atan2(x - cx);
            let end_angle = (y2 - cy).atan2(x2 - cx);
            let mut delta = if is_g3 {
                let mut d = end_angle - start_angle;
                if d <= 0.0 {
                    d += std::f64::consts::PI * 2.0;
                }
                d
            } else {
                let mut d = end_angle - start_angle;
                if d >= 0.0 {
                    d -= std::f64::consts::PI * 2.0;
                }
                d
            };
            if delta.abs() < 1e-6 {
                delta = 0.0;
            }
            let radius = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
            let arc_len = radius * delta.abs();
            let mut steps = (arc_len / 2.0).ceil() as usize;
            if steps < 8 {
                steps = 8;
            }
            let step = if steps > 0 {
                delta / steps as f64
            } else {
                0.0
            };
            let mut px = x;
            let mut py = y;
            for k in 1..=steps {
                let ang = start_angle + step * k as f64;
                let nx = cx + radius * ang.cos();
                let ny = cy + radius * ang.sin();
                segs.push(Seg {
                    x1: px,
                    y1: py,
                    x2: nx,
                    y2: ny,
                    cut: torch_on,
                });
                px = nx;
                py = ny;
            }
            x = x2;
            y = y2;
        } else if x2 != x || y2 != y {
            segs.push(Seg {
                x1: x,
                y1: y,
                x2,
                y2,
                cut: torch_on && is_g1,
            });
            x = x2;
            y = y2;
        }
    }

    segs
}
