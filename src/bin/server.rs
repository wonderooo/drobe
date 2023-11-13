#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use cyw43::PowerManagementMode;
use defmt::*;
use embassy_executor::{Spawner, InterruptExecutor};
use embassy_rp::interrupt::{InterruptExt, Priority};
use embassy_rp::interrupt;
use lib::rlog::RemoteLog;
use lib::stepper::{Motor, TurnSteps, Ticks, global_turn};
use lib::net::{Wlan, WlanCredentials, WlanPins, Ipv4Config, Ipv4WithMask, Ipv4};
use {defmt_rtt as _, panic_probe as _};

// ---------------------------------
// UNCOMMENT ONLY FOR DEBUG PURPOSES 
// ---------------------------------
//
// use lib::{rlog, rwarn, rerror};
// use lib::rlog::{log, color::Color};

static EXECUTOR_HIGH: InterruptExecutor = InterruptExecutor::new();

#[interrupt]
unsafe fn SWI_IRQ_3() {
    EXECUTOR_HIGH.on_interrupt()
}


#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let wlan = Wlan::new(WlanPins::new(p.PIN_23, p.PIN_25, p.PIO0, p.PIN_24, p.PIN_29, p.DMA_CH0))
        .with_credentials(WlanCredentials::new("FELIX", None))
        .with_static_address(Ipv4Config::new(Ipv4WithMask([192, 168, 4, 159], 24), Some(Ipv4([192, 168, 4, 1]))))
        .with_power_mode(PowerManagementMode::None)
        .connect().await;

    interrupt::SWI_IRQ_3.set_priority(Priority::P3);
    let spawner_interrupt = EXECUTOR_HIGH.start(interrupt::SWI_IRQ_3);

    let rl = RemoteLog::new(wlan.stack, 3333);
    let motor = Motor::new(Default::default(), p.PIN_4, p.PIN_3, p.PIN_5);

    unwrap!(spawner.spawn(rl.init()));
    unwrap!(spawner_interrupt.spawn(global_turn(motor, TurnSteps(200 * 1000), Ticks(3))));
}

