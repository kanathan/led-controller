use anyhow::Result;
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    nvs::EspDefaultNvsPartition,
};

use std:: {
    thread,
    time::Duration,
};

// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;

mod effects;
mod led_control;
mod ota;
mod server;
mod wifi;

fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();
    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    // Macro to update app version info
    esp_idf_sys::esp_app_desc!();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let wifi_svc = wifi::WifiService::run_wifi_service(peripherals.modem, sysloop, nvs)?;

    let _server = server::ServerService::init_server(wifi_svc)?;

    let _led_ctrl = led_control::LEDControllerService::init(
        peripherals.rmt.channel0,
        peripherals.pins.gpio15
    )?;

    loop {
        thread::sleep(Duration::from_secs(1000));
    }
}
