use core::str;

use anyhow::{bail, Result};
use esp32c3_wifi::wifi;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::prelude::Peripherals,
    http::{
        client::{self, Configuration as HttpConfiguration, EspHttpConnection},
        Method,
    },
    wifi::{
        AuthMethod, BlockingWifi, ClientConfiguration, Configuration as WifiConfiguration, EspWifi,
    },
};
use log::{debug, info};

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;

    let ssid = "test";
    let pass = "";
    let auth_method = AuthMethod::None;
    let _wifi = wifi(ssid, pass, auth_method, peripherals.modem, sysloop)?;
    get("http://neverssl.com/")?;
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn get(url: impl AsRef<str>) -> Result<()> {
    let config = HttpConfiguration::default();
    let connection = EspHttpConnection::new(&config)?;
    let mut client = embedded_svc::http::client::Client::wrap(connection);
    let headers = [("accept", "text/plain")];

    let request = client.request(Method::Get, url.as_ref(), &headers)?;
    let response = request.submit()?;
    let status = response.status();
    println!("Response code: {}\n", status);
    match status {
        200..=299 => {
            let mut buf = [0_u8; 256];
            let mut offset = 0;
            let mut total = 0;
            let mut reader = response;
            loop {
                if let Ok(size) = embedded_svc::io::Read::read(&mut reader, &mut buf[offset..]) {
                    if size == 0 {
                        break;
                    }
                    total += size;
                    let size_plus_offset = size + offset;
                    match str::from_utf8(&buf[..size_plus_offset]) {
                        Ok(text) => {
                            print!("{}", text);
                            offset = 0;
                        }
                        Err(error) => {
                            let valid_up_to = error.valid_up_to();
                            unsafe {
                                print!("{}", str::from_utf8_unchecked(&buf[..valid_up_to]));
                            }
                            buf.copy_within(valid_up_to.., 0);
                            offset = size_plus_offset - valid_up_to;
                        }
                    }
                }
            }
            println!("Total: {} bytes", total);
        }
        _ => bail!("Unexpected response code: {}", status),
    }

    Ok(())
}
