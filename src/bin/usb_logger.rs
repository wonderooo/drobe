//! This example shows how to use USB (Universal Serial Bus) in the RP2040 chip.
//!
//! This creates the possibility to send log::info/warn/error/debug! to USB serial port.

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embassy_executor::Spawner;
use embassy_time::{Timer, Duration};
use lib::ulog::UsbLog;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    UsbLog::init(p.USB).await;

    let mut counter = 0;
    loop {
        counter += 1;

        log::info!("Tick info {}", counter);
        log::warn!("Tick warn {}", counter);
        log::debug!("Tick debug {}", counter);

        Timer::after(Duration::from_secs(1)).await;
    }
}