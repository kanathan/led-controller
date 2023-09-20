use anyhow::Result;
use esp_idf_svc::http::server::{
    EspHttpServer,
    Configuration,  
};
use embedded_svc::http::Method;

const LANDING_HTML: &str = include_str!("../data/landing.html");


pub fn init_server() -> Result<Box<EspHttpServer>> {
    let mut server = EspHttpServer::new(&Configuration::default())?;

    server.fn_handler("/", Method::Get, |request| {
        let mut response = request.into_ok_response()?;
        response.write(LANDING_HTML.as_bytes())?;
        Ok(())
    })?;

    Ok(Box::new(server))
}
