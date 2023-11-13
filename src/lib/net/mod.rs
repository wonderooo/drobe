use cyw43::{State, Control, PowerManagementMode};
use cyw43_pio::PioSpi;
use defmt::unwrap;
use embassy_executor::Spawner;
use embassy_net_wiznet::Device;
use embassy_rp::{gpio::Output, peripherals::{PIN_23, PIN_25, PIO0, DMA_CH0, PIN_24, PIN_29}, gpio::Level, pio::Pio};
use embassy_net::{Stack, Config, StackResources, Ipv4Cidr, StaticConfigV4, Ipv4Address};
use embassy_time::{Timer, Duration};
use heapless::Vec;
use static_cell::make_static;

use crate::{StackType, Irqs};

mod consts;

#[embassy_executor::task]
async fn wifi_task(
    runner: cyw43::Runner<'static, Output<'static, PIN_23>, PioSpi<'static, PIN_25, PIO0, 0, DMA_CH0>>,
) -> ! {    
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<cyw43::NetDriver<'static>>) -> ! {
    stack.run().await
}

pub struct WlanPins {
    pwr: PIN_23,
    cs: PIN_25,
    pio: PIO0,
    spi_pin_1: PIN_24,
    spi_pin_2: PIN_29,
    dma: DMA_CH0,
}

impl WlanPins {
    pub fn new(pwr: PIN_23, cs: PIN_25, pio: PIO0, spi_pin_1: PIN_24, spi_pin_2: PIN_29, dma: DMA_CH0) -> Self {
        Self { pwr, cs, pio, spi_pin_1, spi_pin_2, dma }
    }
}

pub struct WlanCredentials {
    ssid: &'static str,
    password: Option<&'static str>,
}

impl WlanCredentials {
    pub fn new(ssid: &'static str, password: Option<&'static str>) -> Self {
        Self { ssid, password }
    }
}

impl Default for WlanCredentials {
    fn default() -> Self {
        Self { ssid: consts::WIFI_SSID, password: Some(consts::WIFI_PASSWORD) }
    }
}

pub struct Ipv4WithMask(pub [u8; 4], pub u8);

pub struct Ipv4(pub [u8; 4]);

pub struct Ipv4Config {
    pub ip: Ipv4WithMask,
    pub gateway: Option<Ipv4>,
}

impl Ipv4Config {
    pub fn new(ip: Ipv4WithMask, gateway: Option<Ipv4>) -> Self {
        Self { ip, gateway }
    }

    pub fn from_cyw_config(cyw: StaticConfigV4) -> Self {
        let ip = Ipv4WithMask(cyw.address.address().0, cyw.address.prefix_len());

        let gateway = if let Some(g) = cyw.gateway {
            Some(Ipv4(g.0))
        } else {
            None
        };

        Self::new(ip, gateway)
    }
}

pub struct Wlan {
    pub stack: StackType,
    pub credentials: WlanCredentials,
    pub address: Ipv4Config
}

impl Wlan {
    pub fn new (pins: WlanPins) -> WlanConfigurable {
        WlanConfigurable {
            credentials: Default::default(),
            pins,
            power_mode: consts::INIT_POWER_MODE,
            static_address: None
        }
    }

    fn _new(stack: StackType, credentials: WlanCredentials, address: Ipv4Config) -> Self {
        Self { stack, credentials, address}
    }
}

pub struct WlanConfigurable {
    credentials: WlanCredentials,
    pins: WlanPins,
    power_mode: PowerManagementMode,
    static_address: Option<Ipv4Config>
}


impl WlanConfigurable {
    pub fn with_credentials(mut self, credentials: WlanCredentials) -> Self {
        self.credentials = credentials;
        self
    }

    pub fn with_static_address(mut self, config: Ipv4Config) -> Self {
        self.static_address = Some(config);
        self
    }

    pub fn with_power_mode(mut self, mode: PowerManagementMode) -> Self {
        self.power_mode = mode;
        self
    }

    pub async fn connect(self) -> Wlan {
        let spawner = Spawner::for_current_executor().await;

        let pwr = Output::new(self.pins.pwr, Level::Low);
        let cs = Output::new(self.pins.cs, Level::High);
        let mut pio = Pio::new(self.pins.pio, Irqs);
        let spi = PioSpi::new(
            &mut pio.common,
            pio.sm0,
            pio.irq0,
            cs,
            self.pins.spi_pin_1,
            self.pins.spi_pin_2,
            self.pins.dma
        );

        let fw: &[u8; consts::FIRMWARE_BYTES] = include_bytes!("../../../firmware/43439A0.bin");
        let clm: &[u8; consts::FIRMWARE_CLM_BYTES] = include_bytes!("../../../firmware/43439A0_clm.bin");

        let (device, mut control) = Self::make_device(pwr, spi, fw, spawner).await;
        control.init(clm).await;
        control.set_power_management(self.power_mode).await;

        let stack = Self::make_stack(device, &self.static_address, spawner).await;

        if let Err(err) = Self::join_network(control, stack, &self.credentials).await {
            panic!("Failed to join network due to: {:?}", err)
        }

        Wlan::_new(stack, self.credentials, Ipv4Config::from_cyw_config(stack.config_v4().unwrap()))
    }

    async fn make_device(pwr: Output<'static, PIN_23>, spi: PioSpi<'static, PIN_25, PIO0, 0, DMA_CH0>, fw: &'static[u8; consts::FIRMWARE_BYTES], spawner: Spawner)
        -> (Device<'static>, Control<'static>)
    {
        let state = make_static!(State::new());
        let (device, control, runner) = cyw43::new(state, pwr, spi, fw).await;

        unwrap!(spawner.spawn(wifi_task(runner)));

        (device, control)
    }

    async fn make_stack(device: Device<'static>, config: &Option<Ipv4Config>, spawner: Spawner) -> StackType {
        let config = if let Some(conf) = config {
            let gateway = if let Some(g) = &conf.gateway {
                Some(Ipv4Address::new(g.0[0], g.0[1], g.0[2], g.0[3]))
            } else {
                None
            };

            Config::ipv4_static(StaticConfigV4 {
                address: Ipv4Cidr::new(Ipv4Address::new(conf.ip.0[0], conf.ip.0[1], conf.ip.0[2], conf.ip.0[3]), conf.ip.1),
                gateway,
                dns_servers: Vec::new(),
            })
        } else {
            Config::dhcpv4(Default::default())
        };

        let seed = 0x0123_4567_89ab_cdef;

        let stack = &*make_static!(
            Stack::new(
                device,
                config,
                make_static!(StackResources::<2>::new()),
                seed
            )
        );
        unwrap!(spawner.spawn(net_task(stack)));

        stack
    }

    async fn join_network(mut control: Control<'static>, stack: StackType, credentials: &WlanCredentials) -> Result<(), cyw43::ControlError> {
        if let Some(password) = credentials.password {
            loop {
                match control.join_wpa2(credentials.ssid, password).await {
                    Ok(_) => break,
                    Err(err) => return Err(err)
                }
            }
        } else {
            loop {
                match control.join_open(credentials.ssid).await {
                    Ok(_) => break,
                    Err(err) => return Err(err)
                }
            }
        }

        while !stack.is_config_up() {
            Timer::after(Duration::from_millis(100)).await;
        }

        Ok(())
    }
}
