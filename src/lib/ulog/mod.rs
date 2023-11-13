use defmt::unwrap;
use embassy_executor::Spawner;
use embassy_rp::{usb::Driver, peripherals::USB};

use crate::Irqs;

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::max(), driver);
}

pub struct UsbLog {}

impl UsbLog {
    pub async fn init(usb_pin: USB) -> () {
        let spawner = Spawner::for_current_executor().await;
        
        let driver = Driver::new(usb_pin, Irqs);
        unwrap!(spawner.spawn(logger_task(driver)));
    }
}