#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use embassy_net::Stack;
use embassy_net_wiznet::Device;
use embassy_rp::{bind_interrupts, usb::InterruptHandler as UsbInterruptHandler, peripherals::{USB, PIO0}, pio::InterruptHandler as PioInterruptHandler};

// CAN BE CALLED ONLY ONCE THRU ENTIRE PROGRAM
bind_interrupts!(pub struct Irqs {
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

pub mod stepper;

// LOGGING THRU TCP, USE USB LOGGING WHEREVER YOU CAN
pub mod rlog;

pub mod ulog;

pub mod net;

// ----------
// LIB SHARED
// ----------
pub type StackType = &'static Stack<Device<'static>>;
