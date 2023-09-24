use super::Effect;
use crate::led_control::Color;
use crate::led_control::Segment;

use rand::prelude::*;
use std::time::{Duration, Instant};

pub struct SpookyEyes {
    eye_pairs: Vec<EyePair>,
    last_update: Instant,
}


const FADETIME: Duration = Duration::from_secs(2);


impl SpookyEyes {
    pub fn init(n_eyes: usize, segment_length: usize) -> Self {
        
        // TODO: Logic to split up eyes randomly

        let eye_pairs = vec![
            EyePair::new((15,16)),
            EyePair::new((34,35))
        ];


        Self {
            eye_pairs,
            last_update: Instant::now(),
        }
    }
}

impl Effect for SpookyEyes {
    fn tick(&mut self, segment: &mut Segment) -> anyhow::Result<()> {
        let duration = self.last_update.elapsed();
        self.last_update = Instant::now();

        for eye_pair in self.eye_pairs.iter_mut() {
            eye_pair.tick(duration, segment)
        }

        Ok(())
    }
}


struct EyePair {
    indices: (usize, usize),
    state: EyeState,
    color: Color,
}

impl EyePair {
    fn new(indices: (usize, usize)) -> Self {
        Self {
            indices,
            state: EyeState::set_closed(),
            color: Color::rgb(153, 0, 0),
        }
    }

    fn tick(&mut self, duration: Duration, segment: &mut Segment) {
        let new_color;

        match &mut self.state {
            EyeState::Closed(remaining) => {
                *remaining = remaining.saturating_sub(duration);
                if remaining.is_zero() {
                    self.state = EyeState::set_opening();
                }
                new_color = Color::black();
            },
            EyeState::Opened(remaining) => {
                *remaining = remaining.saturating_sub(duration);
                if remaining.is_zero() {
                    self.state = EyeState::set_closing();
                }
                new_color = self.color;
            },
            EyeState::Opening(remaining) => {
                *remaining = remaining.saturating_sub(duration);
                if remaining.is_zero() {
                    self.state = EyeState::set_opened();
                    new_color = self.color;
                } else {
                    new_color = self.color * (1.0 - remaining.as_secs_f32()/FADETIME.as_secs_f32());
                }
            },
            EyeState::Closing(remaining) => {
                *remaining = remaining.saturating_sub(duration);
                if remaining.is_zero() {
                    self.state = EyeState::set_closed();
                    new_color = Color::black();
                } else {
                    new_color = self.color * (remaining.as_secs_f32()/FADETIME.as_secs_f32());
                }
            }
        }

        for idx in [self.indices.0, self.indices.1] {
            if let Some(led) = segment.leds_mut().get_mut(idx) {
                led.set(new_color);
            }
        }
    }
}


enum EyeState {
    Closed(Duration),
    Opened(Duration),
    Opening(Duration),
    Closing(Duration),
}

impl EyeState {
    fn set_closed() -> Self {
        let secs = thread_rng().gen_range(1..=10);
        log::info!("Setting closed for {secs}s");
        EyeState::Closed(Duration::from_secs(secs))
    }

    fn set_opened() -> Self {
        let secs = thread_rng().gen_range(5..=30);
        log::info!("Setting opened for {secs}s");
        EyeState::Opened(Duration::from_secs(secs))
    }

    fn set_opening() -> Self {
        log::info!("Opening eyes");
        EyeState::Opening(FADETIME)
    }

    fn set_closing() -> Self {
        log::info!("Closing eyes");
        EyeState::Closing(FADETIME)
    }
}