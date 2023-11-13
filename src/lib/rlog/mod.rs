pub mod color;
pub mod r#macro;

use core::fmt::{write, Debug};

use defmt::{warn, info};
use embassy_executor::SpawnToken;
use embassy_net::tcp::TcpSocket;
use embassy_sync::{channel::Channel, blocking_mutex::raw::CriticalSectionRawMutex};
use embassy_time::{Duration, Instant, with_timeout};
use embedded_io_async::Write;
use heapless::String;

use crate::StackType;

use self::color::Color;

type ChanType<'a> = Channel<CriticalSectionRawMutex, Message<'a>, 4>;


static CHAN: ChanType = Channel::new();

pub struct RemoteLog {
    stack: StackType,
    port: u16,
    rx_buffer: [u8; 4096],
    tx_buffer: [u8; 4096],
}

pub struct Message<'a> {
    msg: &'a (dyn Debug + Send + Sync),
    color: Color,
}

impl<'a> Message<'a> {
    fn new(msg: &'a(dyn Debug + Send + Sync), color: Color) -> Self {
        Message { msg, color }
    }

    fn fmt(&self) -> String<48> {
        let mut buf: String<48> = String::new();
        let reset = Color::Reset;
        write(&mut buf,
            format_args!("{}{}{} {}s: {:?}\n",
                self.color.make(), self.color.to_log_severity(), reset.make(), Instant::now().as_secs(), self.msg
            )
        ).unwrap();

        buf
    }
}

impl RemoteLog {
    pub fn new(stack: StackType, port: u16) -> Self {
        RemoteLog {
            stack,
            port,
            rx_buffer: [0; 4096],
            tx_buffer: [0; 4096],
        }
    }

    pub fn init(&self) -> SpawnToken<impl Sized> {
       _init(self.stack, self.port, self.rx_buffer, self.tx_buffer)
    }
}
pub fn log(what: &'static (dyn Debug + Send + Sync), color: color::Color) -> () {
    let _ = CHAN.try_send(Message::new(what, color));
}   



#[embassy_executor::task]
async fn _init(stack: StackType, port: u16, mut rx: [u8; 4096], mut tx: [u8; 4096]) -> () {
    loop {
        let mut socket = TcpSocket::new(stack, &mut rx, &mut tx);
        socket.set_timeout(Some(Duration::from_secs(3)));
        socket.set_keep_alive(Some(Duration::from_secs(3)));

        info!("Remote TCP logging available on port :{}", port);
        if let Err(e) = socket.accept(port).await {
            warn!("Error accepting tcp connection on port :{:?}, err: {:?}", port, e);
            continue;
        }

        loop {
            let msg = CHAN.receive().await;
            
            match socket.write_all(msg.fmt().as_bytes()).await {
                Ok(()) => {
                    match with_timeout(Duration::from_secs(3), socket.flush()).await {
                        Ok(_) => continue,
                        Err(_) => break, //FLUSH ERROR WHEN CLIENT DISCONNECTS
                    }
                },
                Err(_) => break, //WRITE ERROR
            }
        }

        socket.close();
    }
}
