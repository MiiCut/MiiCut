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
        if !is_g0 && !is_g1 {
            continue;
        }

        let mut nx = None::<f64>;
        let mut ny = None::<f64>;

        for tok in up.split_whitespace() {
            if let Some(v) = tok.strip_prefix('X') {
                if let Ok(f) = v.parse::<f64>() {
                    nx = Some(f);
                }
            } else if let Some(v) = tok.strip_prefix('Y') {
                if let Ok(f) = v.parse::<f64>() {
                    ny = Some(f);
                }
            }
        }

        let x2 = nx.unwrap_or(x);
        let y2 = ny.unwrap_or(y);

        if x2 != x || y2 != y {
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
