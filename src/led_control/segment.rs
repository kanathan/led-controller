use super::led::Led;
use super::color::Color;

pub struct Segment {
    leds: Vec<Led>
}

impl Segment {
    pub fn new(led_count: usize) -> Self {
        Self {
            leds: vec![Led::new(); led_count+1] // BUG: Adding +1 to avoid last led from flickering
        }
    }

    #[allow(dead_code)]
    pub fn turn_off(&mut self) {
        for led in self.leds.iter_mut() {
            led.turn_off();
        }
    }

    #[allow(dead_code)]
    pub fn set_all(&mut self, color: Color) {
        for led in self.leds.iter_mut() {
            led.set(color);
        }
    }

    pub fn leds(&self) -> &[Led] {
        &self.leds
    }

    pub fn leds_mut(&mut self) -> &mut [Led] {
        &mut self.leds
    }
}