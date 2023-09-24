use anyhow::Result;

use crate::led_control::Segment;

mod blink;
mod rainbow;
mod spookyeyes;
pub use blink::Blink;
pub use rainbow::Rainbow;
pub use spookyeyes::SpookyEyes;


pub trait Effect {
    fn tick(&mut self, segment: &mut Segment) -> Result<()>;
}