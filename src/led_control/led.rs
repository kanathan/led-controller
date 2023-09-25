use super::color::Color;

#[derive(Default, Clone, Copy)]
pub struct Led {
    color: Color,
}

impl Led {
    pub fn new() -> Self {
        Self {
            color: Color::black()
        }
    }

    #[allow(dead_code)]
    pub fn turn_off(&mut self) {
        self.color = Color::black();
    }

    pub fn set(&mut self, color: Color) {
        self.color = color;
    }

    pub fn color(&self) -> Color {
        self.color
    }
}