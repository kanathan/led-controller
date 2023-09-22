use esp_idf_svc::{
    ota::EspOta,
    http::server::EspHttpConnection,
};
use embedded_svc::http::server::Request;
use anyhow::{Result, Error};


pub fn ota_update(request: &mut Request<&mut EspHttpConnection>) -> Result<()> {

    let mut esp_ota = EspOta::new()?;

    let running_slot = esp_ota.get_running_slot()?;
    let next_slot = esp_ota.get_update_slot()?;

    log::info!("Running OTA slot: {} State: {:?} Firmware: {:?}", running_slot.label, running_slot.state, running_slot.firmware);
    log::info!("Update OTA slot: {} State: {:?} Firmware: {:?}", next_slot.label, next_slot.state, next_slot.firmware);

    let mut remaining = request.header("Content-Length")
        .ok_or(Error::msg("Missing Content-Length"))?
        .parse::<usize>()?;
    log::info!("Receiving {remaining} bytes of data for OTA update");

    let ota_updater = esp_ota.initiate_update()?;
    let mut buffer: [u8; 256] = [0; 256];

    loop {
        let size = match request.read(&mut buffer) {
            Ok(size) => size,
            Err(e) => {
                log::error!("Error receiving data. Aborting update - {}", e.to_string());
                ota_updater.abort()?;
                return Err(e.into())
            }
        };

        remaining = match remaining.checked_sub(size) {
            Some(val) => val,
            None => {
                log::error!("Recieved more data than expected. Aborting");
                ota_updater.abort()?;
                return Err(Error::msg("Content-Length and downloaded size don't match"))
            }
        };

        if remaining > 0 && size == 0 {
            log::error!("Recieved less data than expected. Aborting");
            ota_updater.abort()?;
            return Err(Error::msg("Content-Length and downloaded size don't match"))
        }

        if let Err(e) = ota_updater.write(&buffer[..size]) {
            ota_updater.abort()?;
            return Err(e.into())
        }

        if remaining == 0 { break }
    }
    
    ota_updater.complete()?;
    log::info!("Updating complete!");

    Ok(())
}