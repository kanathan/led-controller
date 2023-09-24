
#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn rgb(r: u8, b: u8, g: u8) -> Self {
        Self {
            r, g, b
        }
    }

    pub fn hsv(h: u8, s: u8, v: u8) -> Self {
        // https://en.wikipedia.org/wiki/HSL_and_HSV
        let h_deg = (h as f32) / 255.0 * 360.0;
        let s_norm = (s as f32) / 255.0;
        let v_norm = (v as f32) / 255.0;

        let c = v_norm * s_norm;
        let h_dash = h_deg / 60.0;
        let x = c * (1.0 - f32::abs(h_dash % 2.0 - 1.0));

        let (r1, g1, b1) =
            if h_dash < 1.0      { (c, x, 0.0) }
            else if h_dash < 2.0 { (x, c, 0.0) }
            else if h_dash < 3.0 { (0.0, c, x) }
            else if h_dash < 4.0 { (0.0, x, c) }
            else if h_dash < 5.0 { (x, 0.0, c) }
            else if h_dash < 6.0 { (c, 0.0, x) }
            else {
                //shouldn't be reachable
                (0.0, 0.0, 0.0)
            };
        
        let m = v_norm - c;
        
        Self {
            r: ((r1 + m) * 255.0) as u8,
            g: ((g1 + m) * 255.0) as u8,
            b: ((b1 + m) * 255.0) as u8,
        }
    }

    pub fn black() -> Self {
        Self::rgb(0, 0, 0)
    }

    pub fn white() -> Self {
        Self::rgb(255, 255, 255)
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::black()
    }
}

impl std::ops::Mul<f32> for Color {
    type Output = Color;
    fn mul(self, rhs: f32) -> Color {
        Color {
            r: (self.r as f32 * rhs).round() as u8,
            g: (self.g as f32 * rhs).round() as u8,
            b: (self.b as f32 * rhs).round() as u8
        }
    }
}