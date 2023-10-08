use super::Effect;
use crate::led_control::Color;
use crate::led_control::Segment;

use rand::prelude::*;
use rand::distributions::WeightedIndex;
use std::time::{Duration, Instant};

pub struct SpookyEyes {
    eye_pairs: Vec<EyePair>,
    last_update: Instant,
}

const COLOR_CORRECTION: (u8, u8, u8) = (255, 224, 140);

const INIT_ON_TIME: Duration = Duration::from_secs(15);
const FADETIME: Duration = Duration::from_secs(2);
const BLINKTIME_MS_MIN:  u64 = 200;
const BLINKTIME_MS_MAX: u64 = 500;


const COLOR_WEIGHTS: [(Color, u32); 3] = [
    (Color::rgb(153, 0, 0), 80),    // Red, 80%
    (Color::rgb(178, 115, 0), 15),  // Orange, 15%
    (Color::rgb(51, 102, 0), 5),   // Green, 5%
];

lazy_static::lazy_static! {
    static ref COLOR_DIST: WeightedIndex<u32> = WeightedIndex::new(COLOR_WEIGHTS.iter().map(|item| item.1)).unwrap();
}


impl SpookyEyes {
    pub fn init(segment_length: usize) -> Self {
        let mut eye_pairs = vec![];


        let mut idx = 0;
        let mut color_idx = 0;
        while (idx + 1) < segment_length {
            eye_pairs.push(EyePair::new((idx, idx+1)));
            idx += 8; // Good spread with current setup in tree
            color_idx = (color_idx + 1) % COLOR_WEIGHTS.len();
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
        let color = COLOR_WEIGHTS[COLOR_DIST.sample(&mut thread_rng())].0;

        Self::new_with_color(indices, color)
    }

    fn new_with_color(indices: (usize, usize), color: Color) -> Self {
        let color = Color::rgb(
            (color.r as f32 * (COLOR_CORRECTION.0 as f32) / 255.0).round() as u8,
            (color.g as f32 * (COLOR_CORRECTION.1 as f32) / 255.0).round() as u8,
            (color.b as f32 * (COLOR_CORRECTION.2 as f32) / 255.0).round() as u8);

        Self {
            indices,
            state: EyeState::set_opened_for(INIT_ON_TIME),
            color,
        }
    }

    fn tick(&mut self, duration: Duration, segment: &mut Segment) {
        let new_color;

        match &mut self.state {
            EyeState::Closed(remaining) => {
                *remaining = remaining.saturating_sub(duration);
                if remaining.is_zero() {
                    self.color = COLOR_WEIGHTS[COLOR_DIST.sample(&mut thread_rng())].0;
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
                    let blink_time = rand::thread_rng().gen_range(BLINKTIME_MS_MIN..=BLINKTIME_MS_MAX);
                    self.state = EyeState::Blinking(*remaining_on, Duration::from_millis(blink_time));
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
        let secs_on = thread_rng().gen_range(120..=300);
        let secs_blink = thread_rng().gen_range(2..=30);
        EyeState::Opened(Duration::from_secs(secs_on), Duration::from_secs(secs_blink))
    }

    fn set_opened_for(on_duration: Duration) -> Self {
        let secs_blink = thread_rng().gen_range(2..=30);
        EyeState::Opened(on_duration, Duration::from_secs(secs_blink))
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