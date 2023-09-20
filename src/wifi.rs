use anyhow::Result;
use esp_idf_hal::peripheral;
use esp_idf_svc::{
    eventloop::{EspSystemEventLoop, EspSubscription, System, Wait},
    wifi::{EspWifi, BlockingWifi, WifiEvent, WifiDriver},
    nvs::{EspNvsPartition, NvsDefault},
    timer::EspTaskTimerService,
    netif::{NetifConfiguration, EspNetif, NetifStack},
};
use embedded_svc::{
    wifi::{
        Configuration,
        ClientConfiguration,
        AccessPointConfiguration,
    },
    ipv4,
};
use std:: {
    thread,
    time::Duration,
};


const MAX_RETRIES: u8 = 10;
const AP_SSID: &str = "ESP32";
const AP_PW: &str = "";


enum WifiMode {
    AP,
    Client,
}


pub fn run_wifi_service(
    modem: impl peripheral::Peripheral<P = esp_idf_hal::modem::Modem> + 'static,
    sysloop: EspSystemEventLoop,
    nvs: EspNvsPartition<NvsDefault>,
    timer_service: EspTaskTimerService,
) -> Result<thread::JoinHandle<()>> {

    // AP IP config
    let ipv4_cfg = ipv4::RouterConfiguration {
        subnet: ipv4::Subnet {
            gateway: ipv4::Ipv4Addr::new(192, 168, 1, 1),
            mask: ipv4::Mask(24) // equivalent to 255.255.255.0
        },
        ..Default::default()
    };

    let net_conf = NetifConfiguration {
        ip_configuration: embedded_svc::ipv4::Configuration::Router(ipv4_cfg),
        ..NetifConfiguration::wifi_default_router()
    };

    let driver = WifiDriver::new(modem, sysloop.clone(), Some(nvs))?;
    let esp_wifi = EspWifi::wrap_all(
        driver,
        EspNetif::new(NetifStack::Sta)?,
        EspNetif::new_with_conf(&net_conf)?,
    )?;

    Ok(thread::spawn(move || {
        if let Err(e) = wifi_service_start(esp_wifi, sysloop) {
            log::error!("Error running wifi service: {e:?}");
        }
    }))
}


fn wifi_service_start(
    mut esp_wifi: EspWifi,
    sysloop: EspSystemEventLoop,
) -> Result<()>
{
    // FOR DEBUGGING
    // Subscribe to wifi events
    let _subscription = sysloop.subscribe(move |event: &WifiEvent| {
        on_wifi_event(event);
    })?;

    

    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sysloop.clone())?;

    let mut wifi_mode = match wifi.get_configuration()? {
        Configuration::Client(config) => {
            if config.ssid.is_empty() {
                // No saved ssid
                WifiMode::AP
            } else {
                // Saved ssid, so let's try using that
                log::info!("Will try to connect to previous SSID: {}", config.ssid);
                WifiMode::Client
            }
        },
        // Either no config or in AP mode already
        _ => WifiMode::AP
    };

    log::info!("Starting wifi watchdog");
    loop {
        match wifi_mode {
            WifiMode::Client => {
                while matches!(wifi_mode, WifiMode::Client) {
                    client_watchdog(&mut wifi, &mut wifi_mode)?;
                    thread::sleep(Duration::from_millis(1000));
                }
            },
            WifiMode::AP => {
                activate_ap(&mut wifi)?;
                while wifi.is_up()? && matches!(wifi_mode, WifiMode::AP) {
                    thread::sleep(Duration::from_millis(1000));
                }
            }
        }
        
    }
}


fn on_wifi_event(event: &WifiEvent) {
    log::info!("EVENT: {event:?}");
}


fn client_watchdog(wifi: &mut BlockingWifi<&mut EspWifi>, wifi_mode: &mut WifiMode) -> Result<()> {
    if wifi.is_up()? {
        return Ok(())
    }

    log::info!("Wifi client disconnected. Attempting to reestablish connection");
    
    let mut retries = 0;

    while retries < MAX_RETRIES {
        retries += 1;
        if !wifi.is_started()? {
            match wifi.start() {
                Ok(_) => (),
                Err(e) => {
                    log::warn!("Issue starting wifi: {e:?}");
                    continue
                }
            }
        }
        if !wifi.is_connected()? {
            match wifi.connect() {
                Ok(_) => (),
                Err(e) => {
                    log::warn!("Issue connecting wifi: {e:?}");
                    continue
                }
            }
        }
        match wifi.wait_netif_up() {
            Ok(_) => (),
            Err(e) => {
                log::warn!("Issue with network interface: {e:?}");
                continue
            }
        }
        if wifi.is_up()? {
            return Ok(())
        }
    }

    // Ran out of retries
    log::warn!("Error connecting to wifi client. Returning to AP mode");
    *wifi_mode = WifiMode::AP;

    Ok(())
}


fn activate_ap(wifi: &mut BlockingWifi<&mut EspWifi>) -> Result<()> {
    log::debug!("Activating AP");

    let config = Configuration::AccessPoint(AccessPointConfiguration {
        ssid: AP_SSID.into(),
        password: AP_PW.into(),
        ..Default::default()
    });

    if wifi.is_started()? {
        log::info!("Stopping wifi service to reset config");
        wifi.stop()?;
    }

    wifi.set_configuration(&config)?;

    log::info!("Starting wifi AP service");
    wifi.start()?;

    log::info!("Wifi AP service started. Waiting until fully up");
    wifi.wait_netif_up()?;

    let ip_info = wifi.wifi_mut().ap_netif_mut().get_ip_info()?;
    log::info!("Wifi AP service fully up. Connect at {} on {}", ip_info.ip, AP_SSID);
    
    println!("{:?}", wifi.wifi_mut().ap_netif_mut().get_ip_info()?);
    
    Ok(())
}
