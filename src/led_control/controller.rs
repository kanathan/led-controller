use crate::effects::*;
use super::segment::Segment;
use super::color::Color;
use super::LED_COUNT;

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
    task::thread::ThreadSpawnConfiguration,
    cpu::Core,
};

use std::time::Duration;
use std::sync::OnceLock;
use std::thread;


const CLOCK_DIV: u8 = 8; // 10MHz with an 80 MHz clock on the ESP32
const REFRESH_RATE: Duration = Duration::from_millis(20);

// From WS2811 datasheet https://cdn-shop.adafruit.com/datasheets/WS2811.pdf
const T0H: Duration = Duration::from_nanos(500);
const T1H: Duration = Duration::from_nanos(1200);
const T0L: Duration = Duration::from_nanos(2000);
const T1L: Duration = Duration::from_nanos(1300);
const RES: Duration = Duration::from_micros(60); // Add 10 us margin


// Initialized on first instance of LEDController being created
static PULSES_HIGH: OnceLock<[Pulse; 2]> = OnceLock::new();
static PULSES_LOW: OnceLock<[Pulse; 2]> = OnceLock::new();
static PULSE_RESET: OnceLock<[Pulse; 2]> = OnceLock::new();


pub struct LEDControllerService {
    _handle: thread::JoinHandle<()>,
}


impl LEDControllerService {
    pub fn init<C: RmtChannel>(
        channel: impl peripheral::Peripheral<P = C> + 'static,
        pin: impl peripheral::Peripheral<P = impl OutputPin> + 'static
    ) -> Result<Self> {

        let mut led_controller = LEDController::new(channel, pin)?;

        ThreadSpawnConfiguration {
            name: Some(b"Led_Controller\0"),
            priority: 10,
            pin_to_core: Some(Core::Core0),
            ..Default::default()
        }.set().unwrap();

        let join_handle = thread::spawn(move || {
            if let Err(e) = led_controller.run() {
                log::error!("Error running led controller service: {e:?}");
            }
        });

        // Set back to defaults.
        ThreadSpawnConfiguration::default().set().unwrap();

        Ok(Self {
            _handle: join_handle,
        })
    }
}


struct LEDController<'a> {
    segment: Segment,
    rmt_tx: TxRmtDriver<'a>,
    effect: Box<dyn Effect + Send>,
}


impl<'a> LEDController<'a> {
    fn new<C: RmtChannel>(
        channel: impl peripheral::Peripheral<P = C> + 'a,
        pin: impl peripheral::Peripheral<P = impl OutputPin> + 'a
    ) -> Result<Self> {
        let config = TransmitConfig {
            clock_divider: CLOCK_DIV,
            mem_block_num: 8,
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

        if PULSE_RESET.set([
            Pulse::new_with_duration(hertz, PinState::Low, &RES)?,
            Pulse::new_with_duration(hertz, PinState::Low, &Duration::from_micros(1))?,
        ]).is_err() {
            log::warn!("PULSE_RESET already set")
        }

        //let effect = Box::new(Rainbow::init(0, 10));
        let effect = Box::new(crate::effects::SpookyEyes::init(LED_COUNT));

        Ok(Self {
            segment: Segment::new(LED_COUNT),
            rmt_tx,
            effect,
        })
    }

    fn run(&mut self) -> Result<()> {
        loop {
            thread::sleep(REFRESH_RATE);
            self.tick()?;
        }
    }

    pub fn tick(&mut self) -> Result<()> {

        self.effect.tick(&mut self.segment)?;

        for led in self.segment.leds_mut() {
            led.set(Color::rgb(led.color().r / 2, led.color().g / 2, led.color().b / 2));
        }

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
    for val in [color.r, color.g, color.b] {
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