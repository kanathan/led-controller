use super::Effect;
use crate::led_control::Color;
use crate::led_control::Segment;

use rand::prelude::*;
use std::time::{Duration, Instant};

pub struct SpookyEyes {
    eye_pairs: Vec<EyePair>,
    last_update: Instant,
}

const COLOR_CORRECTION: (u8, u8, u8) = (255, 224, 140);

const FADETIME: Duration = Duration::from_secs(2);
const BLINKTIME: Duration = Duration::from_millis(500);

const COLORS: [Color; 2] = [
    Color::rgb(153, 0, 0), // Red
    //Color::rgb(255, 128, 0), // Orange
    //Color::rgb(204, 204, 0), // Yellow
    Color::rgb(102, 204, 0), // Green
];


impl SpookyEyes {
    pub fn init(n_eyes: usize, segment_length: usize) -> Self {
        let mut eye_pairs = vec![];

        if segment_length < 2 {
            log::error!("Need segment length > 1 for spooky eyes to work");
            return Self {
                eye_pairs,
                last_update: Instant::now()
            }
        }        

        #[allow(clippy::single_range_in_vec_init)]
        let mut free_space = vec![(0..segment_length-1)];

        for _ in 0..n_eyes {
            if free_space.is_empty() { break }

            let rng_idx = thread_rng().gen_range(0..free_space.len());
            let in_range = free_space.swap_remove(rng_idx);
            let min = in_range.start;
            let max = in_range.end - 1;

            let x1 = thread_rng().gen_range(in_range);
            let x2 = x1 + 1;

            if x1 > min + 2 {
                free_space.push(min..(x1-2));
            }
            if x2 + 2 <= max {
                free_space.push(x2+2..(max+1));
            }

            eye_pairs.push(EyePair::new((x1, x2)));
        }

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
        let color = COLORS[thread_rng().gen_range(0..COLORS.len())];

        let color = Color::rgb(
            (color.r as f32 * (COLOR_CORRECTION.0 as f32) / 255.0).round() as u8,
            (color.g as f32 * (COLOR_CORRECTION.1 as f32) / 255.0).round() as u8,
            (color.b as f32 * (COLOR_CORRECTION.2 as f32) / 255.0).round() as u8);

        Self {
            indices,
            state: EyeState::set_closed(),
            color,
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
            EyeState::Opened(remaining_on, remaining_blink) => {
                *remaining_on = remaining_on.saturating_sub(duration);
                *remaining_blink = remaining_blink.saturating_sub(duration);
                if remaining_on.is_zero() {
                    self.state = EyeState::set_closing();
                } else if remaining_blink.is_zero() {
                    self.state = EyeState::Blinking(*remaining_on, BLINKTIME);
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
            },
            EyeState::Blinking(remaining_on, remaining_blink) => {
                // Don't decrement remaining_on at this time
                *remaining_blink = remaining_blink.saturating_sub(duration);
                if remaining_blink.is_zero() {
                    self.state = EyeState::set_reopened(*remaining_on);
                    new_color = self.color;
                } else {
                    new_color = Color::black();
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
    Opened(Duration, Duration),
    Opening(Duration),
    Closing(Duration),
    Blinking(Duration, Duration),
}

impl EyeState {
    fn set_closed() -> Self {
        let secs = thread_rng().gen_range(30..=60);
        EyeState::Closed(Duration::from_secs(secs))
    }

    fn set_opened() -> Self {
        let secs_on = thread_rng().gen_range(120..=180);
        let secs_blink = thread_rng().gen_range(2..=30);
        EyeState::Opened(Duration::from_secs(secs_on), Duration::from_secs(secs_blink))
    }

    fn set_opening() -> Self {
        EyeState::Opening(FADETIME)
    }

    fn set_closing() -> Self {
        EyeState::Closing(FADETIME)
    }

    fn set_reopened(remaining_on: Duration) -> Self {
        let secs_blink = thread_rng().gen_range(2..=30);
        EyeState::Opened(remaining_on, Duration::from_secs(secs_blink))
    }
}