#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use defmt::unwrap;
use embassy_executor::{Spawner, InterruptExecutor};
use embassy_rp::interrupt;
use embassy_rp::interrupt::{InterruptExt, Priority};
use lib::stepper::{Motor, TurnSteps, Ticks};
use {defmt_rtt as _, panic_probe as _};

static EXECUTOR: InterruptExecutor = InterruptExecutor::new();

#[interrupt]
unsafe fn SWI_IRQ_1() {
    EXECUTOR.on_interrupt()
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let motor = Motor::new(Default::default(), p.PIN_4, p.PIN_3, p.PIN_5);

    interrupt::SWI_IRQ_1.set_priority(Priority::P2);
    let spawner = EXECUTOR.start(interrupt::SWI_IRQ_1);
    unwrap!(spawner.spawn(global_turn(motor, TurnSteps(200), Ticks(2))));

    loop {}
}

#[embassy_executor::task]
async fn global_turn(mut motor: Motor, ts: TurnSteps, t: Ticks) {
    let _ = motor.start_turning(ts, t).await;
}
