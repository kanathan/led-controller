use anyhow::Result;
use esp_idf_svc::http::server::{
    EspHttpServer,
    Configuration,
    EspHttpConnection,
};
use embedded_svc::{
    http::Method,
    http::server::Request,
    wifi::ClientConfiguration,
};
use core::str;

use std::collections::HashMap;

use crate::wifi::WifiService;

const LANDING_HTML: &str = include_str!("../data/landing.html");


pub struct ServerService {
    _esp_server: EspHttpServer,
    _wifi_svc: WifiService,
}

impl ServerService {
    pub fn init_server(wifi_svc: WifiService) -> Result<Self> {
        let mut esp_server = EspHttpServer::new(&Configuration::default())?;

        esp_server.fn_handler("/", Method::Get, |request| {
            let mut response = request.into_ok_response()?;
            response.write(LANDING_HTML.as_bytes())?;
            Ok(())
        })?;

        let wifi_sender = wifi_svc.wifi_mode_tx.clone();
        esp_server.fn_handler("/wifi-data", Method::Post, move |mut request| {
            let map =  parse_form_body(&mut request);

            if map.contains_key("ssid") && map.contains_key("password") {
                match wifi_sender.send(crate::wifi::WifiMode::Client(ClientConfiguration {
                    ssid: map.get("ssid").unwrap().as_str().into(),
                    password: map.get("password").unwrap().as_str().into(),
                    ..Default::default()
                })) {
                    Ok(_) => request.into_ok_response()?,
                    Err(_) => request.into_response(500, Some("Unable to send response"), &[])?
                };
            } else {
                request.into_response(400, Some("Bad form data"), &[])?;
            }

            Ok(())
        })?;

        Ok(Self {
            _esp_server: esp_server,
            _wifi_svc: wifi_svc,
        })
    }

}


fn parse_form_body(request: &mut Request<&mut EspHttpConnection>) -> HashMap<String, String> {
    let mut map = HashMap::new();

    let full_string = read_into_string(request);
    for item_str in full_string.split('&') {
        if let Some((key, value)) = item_str.split_once('=') {
            map.insert(key.to_string(), value.to_string());
        }
    }

    map
}


fn read_into_string(request: &mut Request<&mut EspHttpConnection>) -> String {
    let mut buffer: [u8; 256] = [0; 256];
    let mut output = String::new();
    let mut offset = 0;

    loop {
        if let Ok(size) = request.read(&mut buffer[offset..]) {
            if size == 0 { break }
            let size_plus_offset = size + offset;
            match str::from_utf8(&buffer[..size_plus_offset]) {
                Ok(text) => {
                    output.push_str(text);
                    offset = 0;
                },
                Err(error) => {
                    let valid_up_to = error.valid_up_to();
                    let text = str::from_utf8(&buffer[..valid_up_to]).unwrap();
                    output.push_str(text);
                    buffer.copy_within(valid_up_to.., 0);
                    offset = size_plus_offset - valid_up_to;
                }
            }
        }
    }

    output
}