use anyhow::Result;
use esp_idf_hal::peripheral;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    wifi::{EspWifi, BlockingWifi, WifiEvent, WifiDriver},
    nvs::{EspNvsPartition, NvsDefault},
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
    sync::{Arc, Mutex},
};


const MAX_RETRIES: u8 = 10;
const AP_SSID: &str = "ESP32";
const AP_PW: &str = "";

const WIFI_MODE_MUT_ERR: &str = "Failure to unlock WifiMode mutex";


#[derive(Clone, Debug)]
pub enum WifiMode {
    AP,
    Client(ClientConfiguration),
}

enum ClientWatchdogResult {
    Continue,
    SwitchMode(WifiMode),
}


pub struct WifiService {
    _handle: thread::JoinHandle<()>,
    wifi_mode: Arc<Mutex<WifiMode>>,
}


impl WifiService {
    pub fn run_wifi_service(
        modem: impl peripheral::Peripheral<P = esp_idf_hal::modem::Modem> + 'static,
        sysloop: EspSystemEventLoop,
        nvs: EspNvsPartition<NvsDefault>,
    ) -> Result<Self>
    {
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

        let wifi_mode = Arc::new(Mutex::new(WifiMode::AP));
        let wifi_mode_c = wifi_mode.clone();

        let join_handle = thread::spawn(move || {
            if let Err(e) = wifi_service_start(esp_wifi, sysloop, wifi_mode_c) {
                log::error!("Error running wifi service: {e:?}");
            }
        });

        Ok(Self {
            _handle: join_handle,
            wifi_mode,
        })
    }

    pub fn set_wifi_mode(&self, mode: WifiMode) {
        log::info!("Setting wifi mode to {mode:?}");
        *self.wifi_mode.lock().expect(WIFI_MODE_MUT_ERR) = mode;
    }

    pub fn get_wifi_mode(&self) -> WifiMode {
        self.wifi_mode.lock().expect(WIFI_MODE_MUT_ERR).to_owned()
    }
}





fn wifi_service_start(
    mut esp_wifi: EspWifi,
    sysloop: EspSystemEventLoop,
    wifi_mode: Arc<Mutex<WifiMode>>,
) -> Result<()>
{
    // FOR DEBUGGING
    // Subscribe to wifi events
    let _subscription = sysloop.subscribe(move |event: &WifiEvent| {
        on_wifi_event(event);
    })?;

    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sysloop.clone())?;

    *wifi_mode.lock().expect(WIFI_MODE_MUT_ERR) = 
        match wifi.get_configuration()? {
            Configuration::Client(config) => {
                if config.ssid.is_empty() {
                    // No saved ssid
                    WifiMode::AP
                } else {
                    // Saved ssid, so let's try using that
                    log::info!("Will try to connect to previous SSID: {}", config.ssid);
                    WifiMode::Client(config)
                }
            },
            // Either no config or in AP mode already
            _ => WifiMode::AP
        };

    log::info!("Starting wifi watchdog");
    loop {
        let cur_wifi_mode = wifi_mode.lock().expect(WIFI_MODE_MUT_ERR).to_owned();
        match cur_wifi_mode {
            WifiMode::Client(config) => {
                log::info!("Client watchdog");
                wifi.stop()?;
                wifi.set_configuration(&Configuration::Client(config.clone()))?;

                loop {
                    if !wifi.is_up()? {
                        log::info!("Wifi client disconnected. Attempting to connect");
                        let status = perform_client_connection(&mut wifi)?;
                        if let ClientWatchdogResult::SwitchMode(mode) = status {
                            *wifi_mode.lock().expect(WIFI_MODE_MUT_ERR) = mode;
                            break
                        }
                    }
                    if !matches!(&mut *wifi_mode.lock().expect(WIFI_MODE_MUT_ERR), WifiMode::Client(_)) {
                        log::info!("No longer in Client mode");
                        break
                    }
                    thread::sleep(Duration::from_millis(1000));
                }
            },
            WifiMode::AP => {
                log::info!("AP watchdog");
                activate_ap(&mut wifi)?;
                loop {
                    if !wifi.is_up()? || !matches!(&mut *wifi_mode.lock().expect(WIFI_MODE_MUT_ERR), WifiMode::AP) {
                        log::info!("No longer in AP mode or had a disconnection");
                        break
                    }
                    thread::sleep(Duration::from_millis(1000));
                }
            }
        }
        
    }
    log::warn!("Exiting wifi watchdog");
}


fn on_wifi_event(event: &WifiEvent) {
    log::info!("EVENT: {event:?}");
}


fn perform_client_connection(wifi: &mut BlockingWifi<&mut EspWifi>) -> Result<ClientWatchdogResult> {    
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
            return Ok(ClientWatchdogResult::Continue)
        }
    }

    // Ran out of retries
    log::warn!("Error connecting to wifi client. Returning to AP mode");
    Ok(ClientWatchdogResult::SwitchMode(WifiMode::AP))
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
