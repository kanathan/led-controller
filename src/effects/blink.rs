use super::Effect;
use crate::led_control::Color;

use std::time::{Duration, Instant};

pub struct Blink {
    last_update: Instant
}


impl Blink {
    pub fn init() -> Self {
        Self {
            last_update: Instant::now()
        }
    }
}

impl Effect for Blink {
    fn tick(&mut self, segment: &mut crate::led_control::Segment) -> anyhow::Result<()> {
        if self.last_update.elapsed() > Duration::from_secs(1) {
            self.last_update = Instant::now();
            let color = Color::rgb(rand::random(), rand::random(), rand::random());
            for led in segment.leds_mut() {
                led.set(color);
            }
        }

        Ok(())
    }
}


