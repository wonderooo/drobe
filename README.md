## Playing with embedded Rust

### Project works with boards based based on Cortex 0+

### What is implemented
1. Wrapper of wlan driver based on embassy-rs - ability to connect board to wifi network
2. Rlog - remote logging through tcp (this is on top os wlan wrapper)
3. Interface to using 4 pin stepper motors
4. Usb logging - writing logs to host that's connected by USB cable

### Examples
1. `cargo run --release --bin revolute`
2. `cargo run --release --bin server` 
3. `cargo run --release --bin usb-logger`