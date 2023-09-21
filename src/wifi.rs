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
        AuthMethod,
    },
    ipv4,
};
use std:: {
    thread,
    time::Duration,
    sync::{Arc, Mutex, mpsc},
};


const AP_SSID: &str = "ESP32";
const AP_PW: &str = "myesp123";
const AP_AUTH: AuthMethod = AuthMethod::WPA2Personal;
const AP_SUBNET: ipv4::Subnet = ipv4::Subnet {
    gateway: ipv4::Ipv4Addr::new(192, 168, 1, 1),
    mask: ipv4::Mask(24) // equivalent to 255.255.255.0
};

const MODE_MUTEX_ERR: &str = "Failed to unlock wifi mode mutex";


#[derive(Clone, Debug)]
pub enum WifiMode {
    AP,
    Client(ClientConfiguration),
}

pub struct WifiService {
    _handle: thread::JoinHandle<()>,
    pub wifi_mode_tx: mpsc::Sender<WifiMode>,
    cur_mode: Arc<Mutex<WifiMode>>,
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
            subnet: AP_SUBNET,
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

        let cur_mode = Arc::new(Mutex::new(WifiMode::AP));
        let cur_mode_c = cur_mode.clone();
        let (wifi_mode_tx, wifi_mode_rx) = mpsc::channel::<WifiMode>();


        let join_handle = thread::Builder::new()
            .stack_size(4096)
            .spawn(move || {
                if let Err(e) = wifi_service_start(esp_wifi, sysloop, cur_mode_c, wifi_mode_rx) {
                    log::error!("Error running wifi service: {e:?}");
                }
            })?;

        Ok(Self {
            _handle: join_handle,
            wifi_mode_tx,
            cur_mode,
        })
    }

    pub fn current_mode(&self) -> &Arc<Mutex<WifiMode>> {
        &self.cur_mode
    }

    pub fn connect_to_client(&self, ssid: &str, password: &str) -> Result<()> {
        self.wifi_mode_tx.send(WifiMode::Client(ClientConfiguration {
            ssid: ssid.into(),
            password: password.into(),
            ..Default::default()
        }))?;
        Ok(())
    }
}





fn wifi_service_start(
    mut esp_wifi: EspWifi,
    sysloop: EspSystemEventLoop,
    cur_mode: Arc<Mutex<WifiMode>>,
    wifi_mode_rx: mpsc::Receiver<WifiMode>,
) -> Result<()>
{
    // FOR DEBUGGING
    // Subscribe to wifi events
    let _subscription = sysloop.subscribe(move |event: &WifiEvent| {
        on_wifi_event(event);
    })?;

    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sysloop.clone())?;

    let mut commanded_mode_change = 
        match wifi.get_configuration()? {
            Configuration::Client(config) => {
                if config.ssid.is_empty() {
                    // No saved ssid
                    Some(WifiMode::AP)
                } else {
                    // Saved ssid, so let's try using that
                    log::info!("Will try to connect to previous SSID: {}", config.ssid);
                    Some(WifiMode::Client(config))
                }
            },
            // Either no config or in AP mode already
            _ => Some(WifiMode::AP)
        };

    let mut cur_retries: u8 = 0;
    let mut max_retries: u8 = 0;
    log::info!("Starting wifi watchdog");
    loop {
        if let Ok(mode) = wifi_mode_rx.recv_timeout(Duration::from_millis(1000)) {
            // Allow external commands to overwrite a pending mode change
            commanded_mode_change = Some(mode);
        }

        if let Some(mode) = commanded_mode_change.take() {
            cur_retries = 0;

            log::info!("Switching wifi modes: {mode:?}");
            *cur_mode.lock().expect(MODE_MUTEX_ERR) = mode.clone();
            wifi.stop()?;
            match mode {
                WifiMode::Client(config) => {
                    max_retries = 0; // Don't try connecting more than once
                    wifi.set_configuration(&Configuration::Client(config))?;
                },
                WifiMode::AP => {
                    max_retries = u8::MAX; // Connect forever
                    let config = Configuration::AccessPoint(AccessPointConfiguration {
                        ssid: AP_SSID.into(),
                        password: AP_PW.into(),
                        auth_method: AP_AUTH,
                        ..Default::default()
                    });

                    wifi.set_configuration(&config)?;
                }
            }
        }

        if !wifi.is_up()? {
            if cur_retries > max_retries {
                log::info!("Unable to connect. Switching to AP mode");
                commanded_mode_change = Some(WifiMode::AP);
            } else {
                log::info!("Wifi client disconnected. Attempting to connect");
                perform_wifi_connection(&mut wifi)?;
                cur_retries = cur_retries.saturating_add(1);
            }
        } else {
            cur_retries = 0;
        }
    }
}

fn on_wifi_event(event: &WifiEvent) {
    log::info!("WIFI EVENT: {event:?}");
}


fn perform_wifi_connection(wifi: &mut BlockingWifi<&mut EspWifi>) -> Result<()> {    

    if !wifi.is_started()? {
        match wifi.start() {
            Ok(_) => (),
            Err(e) => {
                log::warn!("Issue starting wifi: {e}");
                return Ok(())
            }
        }
    }
    if !wifi.is_connected()? {
        match wifi.connect() {
            Ok(_) => (),
            Err(e) => {
                log::warn!("Issue connecting wifi: {e}");
                return Ok(())
            }
        }
    }
    match wifi.wait_netif_up() {
        Ok(_) => (),
        Err(e) => {
            log::warn!("Issue with network interface: {e}");
            return Ok(())
        }
    }
    Ok(())
}
