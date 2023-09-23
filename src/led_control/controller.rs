use super::segment::Segment;
use super::color::Color;

use anyhow::{Result, Error};
use esp_idf_hal::{
    rmt::{
        Pulse,
        PinState,
        VariableLengthSignal,
        TxRmtDriver,
        config::{Loop, TransmitConfig},
        RmtChannel
    },
    peripheral,
    gpio::OutputPin,
};

use std::time:: {
    Duration,
    Instant,
};
use std::sync::OnceLock;


const CLOCK_DIV: u8 = 8; // 10MHz with an 80 MHz clock on the ESP32

// From WS2811 datasheet https://cdn-shop.adafruit.com/datasheets/WS2811.pdf
const T0H: Duration = Duration::from_nanos(500);
const T1H: Duration = Duration::from_nanos(1200);
const T0L: Duration = Duration::from_nanos(2000);
const T1L: Duration = Duration::from_nanos(1300);
const RES: Duration = Duration::from_micros(50);


// Initialized on first instance of LEDController being created
static PULSES_HIGH: OnceLock<[Pulse; 2]> = OnceLock::new();
static PULSES_LOW: OnceLock<[Pulse; 2]> = OnceLock::new();
static PULSE_RESET: OnceLock<[Pulse; 1]> = OnceLock::new();


pub struct LEDController<'a> {
    segment: Segment,
    rmt_tx: TxRmtDriver<'a>,
    last_update: Instant,
    active_led: usize,
}


impl<'a> LEDController<'a> {
    pub fn new<C: RmtChannel>(
        channel: impl peripheral::Peripheral<P = C> + 'a,
        pin: impl peripheral::Peripheral<P = impl OutputPin> + 'a
    ) -> Result<Self> {
        let config = TransmitConfig {
            clock_divider: CLOCK_DIV,
            mem_block_num: 1,
            carrier: None,
            looping: Loop::None,
            idle: Some(PinState::Low),
            ..Default::default()
        };

        let rmt_tx = TxRmtDriver::new(channel, pin, &config)?;


        let hertz = rmt_tx.counter_clock()?;

        if PULSES_HIGH.set([
            Pulse::new_with_duration(hertz, PinState::High, &T1H)?,
            Pulse::new_with_duration(hertz, PinState::Low, &T1L)?,
        ]).is_err() {
            log::warn!("PULSES_HIGH already set")
        }

        if PULSES_LOW.set([
            Pulse::new_with_duration(hertz, PinState::High, &T0H)?,
            Pulse::new_with_duration(hertz, PinState::Low, &T0L)?,
        ]).is_err() {
            log::warn!("PULSES_LOW already set")
        }

        if PULSE_RESET.set([Pulse::new_with_duration(hertz, PinState::Low, &RES)?]).is_err() {
            log::warn!("PULSE_RESET already set")
        }


        Ok(Self {
            segment: Segment::new(50),
            rmt_tx,
            last_update: Instant::now(),
            active_led: 0,
        })
    }

    pub fn tick(&mut self) -> Result<()> {
        if self.last_update.elapsed() >= Duration::from_millis(100) {
            self.update()?;
            self.last_update = Instant::now();
        }

        Ok(())
    }

    fn update(&mut self) -> Result<()> {
        for (idx, led) in self.segment.leds_mut().iter_mut().enumerate() {
            if idx == self.active_led {
                led.set(Color::rgb(204, 102, 0));
            } else {
                led.turn_off();
            }
        }
        self.active_led = (self.active_led + 1) % self.segment.leds().len();

        self.send_signal()?;

        Ok(())
    }

    fn send_signal(&mut self) -> Result<()> {
        let mut signal = VariableLengthSignal::new();
        
        for led in self.segment.leds() {
            write_color(&mut signal, led.color())?;
        }

        signal.push(PULSE_RESET.get().unwrap())?;

        self.rmt_tx.start_blocking(&signal)?;

        Ok(())
    }
}


fn write_color(signal: &mut VariableLengthSignal, color: Color) -> Result<()> {
    // For WS2811 sent as RGB with the MSB first
    for val in [color.r, color.b, color.g] {
        for shift in (0..8).rev() {
            if val & (1 << shift) == 0 {
                signal.push(PULSES_LOW.get().ok_or(Error::msg("PULSES_LOW not set"))?)?;
            } else {
                signal.push(PULSES_HIGH.get().ok_or(Error::msg("PULSES_HIGH not set"))?)?;
            }
        }
    }

    Ok(())
}