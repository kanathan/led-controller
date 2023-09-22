use anyhow::Result;
use esp_idf_svc::http::server::{
    EspHttpServer,
    Configuration,
    EspHttpConnection,
};
use embedded_svc::{
    http::Method,
    http::server::Request,
};
use core::str;
use std::collections::HashMap;

use crate::wifi::{WifiService, WifiMode};
use crate::ota;


const LANDING_HTML: &str = include_str!("../data/landing.html");
const FAVICON: &[u8] = include_bytes!("../data/led.ico");


pub struct ServerService {
    _esp_server: EspHttpServer,
    _wifi_svc: WifiService,
}

#[derive(serde::Deserialize)]
struct WifiForm {
    ssid: String,
    password: String,
}

impl ServerService {
    pub fn init_server(wifi_svc: WifiService) -> Result<Self> {
        let mut esp_server = EspHttpServer::new(&Configuration::default())?;



        let wifi_status = wifi_svc.current_mode().clone();
        esp_server.fn_handler("/", Method::Get, move |request| {
            match wifi_status.lock() {
                Ok(status) => {
                    let mut template_data = HashMap::new();
                    let mut response = request.into_ok_response()?;

                    match *status {
                        WifiMode::AP => template_data.insert("wifi_mode", "Access Point(AP)"),
                        WifiMode::Client(_) => template_data.insert("wifi_mode", "Client"),
                    };

                    response.write(replace_template(LANDING_HTML, &template_data).as_bytes())?
                },
                Err(_) => {
                    request.into_response(500, Some("Unable to get wifi status"), &[])?;
                    return Ok(())
                }
            };
            Ok(())
        })?;



        esp_server.fn_handler("/favicon.ico", Method::Get, |request| {
            let mut response = request.into_ok_response()?;
            response.write(FAVICON)?;
            Ok(())
        })?;



        let wifi_sender = wifi_svc.wifi_mode_tx.clone();
        esp_server.fn_handler("/wifi-data", Method::Post, move |mut request| {
            let data = get_request_data(&mut request);

            if let Ok(wifi_form) = serde_urlencoded::from_bytes::<WifiForm>(&data) {
                match wifi_sender.send(crate::wifi::WifiMode::client(&wifi_form.ssid, &wifi_form.password))
                {
                    Ok(_) => request.into_ok_response()?,
                    Err(_) => request.into_response(500, Some("Unable to send response"), &[])?
                };
            } else {
                request.into_response(400, Some("Bad form data"), &[])?;
            }

            Ok(())
        })?;



        esp_server.fn_handler("/ota-update", Method::Post, |mut request| {
            if request.header("X-Requested-With").is_none() {
                log::warn!("ota-update POST without X-Requested-With header");
                request.into_status_response(406)?;
                return Ok(())
            }
            if request.header("X-Requested-With").unwrap() != "XMLHttpRequest" {
                log::warn!("ota-update POST with incorrect X-Requested-With header data: {}", request.header("X-Requested-With").unwrap());
                request.into_status_response(406)?;
                return Ok(())
            }

            match ota::ota_update(&mut request) {
                Ok(_) => request.into_ok_response()?,
                Err(e) => {
                    request.into_response(515, Some(&e.to_string()), &[])?
                }
            };

            Ok(())
        })?;



        Ok(Self {
            _esp_server: esp_server,
            _wifi_svc: wifi_svc,
        })
    }

}


fn get_request_data(request: &mut Request<&mut EspHttpConnection>) -> Vec<u8> {
    let mut output = Vec::new();
    let mut buffer: [u8; 256] = [0; 256];

    loop {
        if let Ok(size) = request.read(&mut buffer) {
            if size == 0 { break }
            output.extend_from_slice(&buffer[..size])
        }
    }

    output
}


fn replace_template(source: &str, data: &HashMap<&str,&str>) -> String {
    let mut output = source.to_string();

    for (key, val) in data.iter() {
        let fullkey = format!("{{{{{key}}}}}");
        output = output.replace(&fullkey, val)
    }

    output
}