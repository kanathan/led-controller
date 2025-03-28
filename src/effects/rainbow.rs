#![allow(dead_code)]

use super::Effect;
use crate::led_control::Color;

pub struct Rainbow {
    deg_per_led: usize,
    deg_per_tick: usize,
    start_deg: usize,
}


impl Rainbow {
    pub fn init(deg_per_led: usize, deg_per_tick: usize) -> Self {
        Self {
            deg_per_led,
            deg_per_tick,
            start_deg: 0
        }
    }
}

impl Effect for Rainbow {
    fn tick(&mut self, segment: &mut crate::led_control::Segment) -> anyhow::Result<()> {
        let mut cur_deg = self.start_deg;
        for led in segment.leds_mut() {
            led.set(get_rgb_from_deg(cur_deg));
            cur_deg = (cur_deg + self.deg_per_led) % 360;
        }

        //self.start_deg = (self.start_deg + self.deg_per_tick) % 360;
        self.start_deg += self.deg_per_tick;
        if self.start_deg >= 360 {
            log::info!("Full loop");
            self.start_deg -= 360;
        }

        Ok(())
    }
}


/*fn get_rgb_from_deg(deg: f32) -> Color {
    let (r, g, b) = 
        if deg < 120.0 {
            ((f32::round((f32::sin((deg + 60.0)*1.5*PI/180.0) + 1.0)/2.0 * 255.0)) as u8,
            (f32::round((f32::sin((deg - 60.0)*1.5*PI/180.0) + 1.0)/2.0 * 255.0)) as u8,
            0)
        } else if deg < 240.0 {
            (0,
            (f32::round((f32::sin((deg - 60.0)*1.5*PI/180.0) + 1.0)/2.0 * 255.0)) as u8,
            (f32::round((f32::sin((deg + 60.0)*1.5*PI/180.0) + 1.0)/2.0 * 255.0)) as u8)
        } else {
            ((f32::round((f32::sin((deg - 60.0)*1.5*PI/180.0) + 1.0)/2.0 * 255.0)) as u8,
            0,
            (f32::round((f32::sin((deg + 60.0)*1.5*PI/180.0) + 1.0)/2.0 * 255.0)) as u8)
        };

    Color::rgb(r, b, g)
}*/


fn get_rgb_from_deg(deg: usize) -> Color {
    Color::rgb(LIGHTS[(deg+120)%360], LIGHTS[deg%360],  LIGHTS[(deg+240)%360])
}



const LIGHTS: [u8; 360] = [
    0,   0,   0,   0,   0,   1,   1,   2, 
    2,   3,   4,   5,   6,   7,   8,   9, 
   11,  12,  13,  15,  17,  18,  20,  22, 
   24,  26,  28,  30,  32,  35,  37,  39, 
   42,  44,  47,  49,  52,  55,  58,  60, 
   63,  66,  69,  72,  75,  78,  81,  85, 
   88,  91,  94,  97, 101, 104, 107, 111, 
  114, 117, 121, 124, 127, 131, 134, 137, 
  141, 144, 147, 150, 154, 157, 160, 163, 
  167, 170, 173, 176, 179, 182, 185, 188, 
  191, 194, 197, 200, 202, 205, 208, 210, 
  213, 215, 217, 220, 222, 224, 226, 229, 
  231, 232, 234, 236, 238, 239, 241, 242, 
  244, 245, 246, 248, 249, 250, 251, 251, 
  252, 253, 253, 254, 254, 255, 255, 255, 
  255, 255, 255, 255, 254, 254, 253, 253, 
  252, 251, 251, 250, 249, 248, 246, 245, 
  244, 242, 241, 239, 238, 236, 234, 232, 
  231, 229, 226, 224, 222, 220, 217, 215, 
  213, 210, 208, 205, 202, 200, 197, 194, 
  191, 188, 185, 182, 179, 176, 173, 170, 
  167, 163, 160, 157, 154, 150, 147, 144, 
  141, 137, 134, 131, 127, 124, 121, 117, 
  114, 111, 107, 104, 101,  97,  94,  91, 
   88,  85,  81,  78,  75,  72,  69,  66, 
   63,  60,  58,  55,  52,  49,  47,  44, 
   42,  39,  37,  35,  32,  30,  28,  26, 
   24,  22,  20,  18,  17,  15,  13,  12, 
   11,   9,   8,   7,   6,   5,   4,   3, 
    2,   2,   1,   1,   0,   0,   0,   0, 
    0,   0,   0,   0,   0,   0,   0,   0, 
    0,   0,   0,   0,   0,   0,   0,   0, 
    0,   0,   0,   0,   0,   0,   0,   0, 
    0,   0,   0,   0,   0,   0,   0,   0, 
    0,   0,   0,   0,   0,   0,   0,   0, 
    0,   0,   0,   0,   0,   0,   0,   0, 
    0,   0,   0,   0,   0,   0,   0,   0, 
    0,   0,   0,   0,   0,   0,   0,   0, 
    0,   0,   0,   0,   0,   0,   0,   0, 
    0,   0,   0,   0,   0,   0,   0,   0, 
    0,   0,   0,   0,   0,   0,   0,   0, 
    0,   0,   0,   0,   0,   0,   0,   0, 
    0,   0,   0,   0,   0,   0,   0,   0, 
    0,   0,   0,   0,   0,   0,   0,   0, 
    0,   0,   0,   0,   0,   0,   0,   0];