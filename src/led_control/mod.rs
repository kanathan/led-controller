mod color;
mod controller;
mod led;
mod segment;

pub use controller::LEDControllerService;
pub use segment::Segment;
pub use led::Led;
pub use color::Color;

const LED_COUNT: usize = 150;