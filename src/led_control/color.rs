
#[derive(Clone, Copy)]
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