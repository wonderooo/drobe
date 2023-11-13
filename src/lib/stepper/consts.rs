pub const STEP_PIN: u8 = 4; // GP04 6th PIN
pub const DIR_PIN: u8 = 3; // GP03 5th PIN
pub const SLEEP_PIN: u8 = 5; // GP05 7th PIN
pub const SPR: usize = 200; // SPR - steps per revolution
pub const ANGLE: f32 = 1.8; // angle of single step: ANGLE * SPR = 360deg
pub const MAX_SPEED: u64 = 450; // max microseconds stepper can handle