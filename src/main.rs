use anyhow::Result;
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    nvs::EspDefaultNvsPartition,
};
use embedded_svc::wifi::ClientConfiguration;
use std:: {
    thread,
    time::Duration,
    time,
};

// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;

mod server;
mod wifi;

fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();
    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // Connect to the Wi-Fi network
    let _wifi_svc = wifi::WifiService::run_wifi_service(peripherals.modem, sysloop, nvs)?;    

    let _server = server::init_server()?;    


    let cur_time = time::Instant::now();
    let mut changed_mode = false;
    let mut changed_mode_2 = false;
    loop {
        if !changed_mode && cur_time.elapsed() > Duration::from_secs(10) {
            println!("Changing to client");
            _wifi_svc.set_wifi_mode(wifi::WifiMode::Client(ClientConfiguration {
                ssid: "RociLANte".into(),
                password: "RememberTheCant".into(),
                ..Default::default()
            }));
            changed_mode = true;
        }
        if !changed_mode_2 && cur_time.elapsed() > Duration::from_secs(20) {
            println!("Changing to bad client");
            _wifi_svc.set_wifi_mode(wifi::WifiMode::Client(ClientConfiguration {
                ssid: "RociLANte".into(),
                password: "RememberTheCant2".into(),
                ..Default::default()
            }));
            changed_mode_2 = true;
        }
        thread::sleep(Duration::from_millis(1000));
    }
}
