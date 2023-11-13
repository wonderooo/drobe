use core::fmt::Debug;

use embassy_rp::gpio::{AnyPin, Output, Pin, Level};
use embassy_time::{Timer, Duration, Instant};

// ---------------------------------
// UNCOMMENT ONLY FOR DEBUG PURPOSES
// ---------------------------------
use crate::rlog;
use crate::rlog::color::Color;
use crate::rlog::log;

mod consts;

pub struct MotorParams {
    pub step_pin: u8,
    pub dir_pin: u8,
    pub sleep_pin: u8,
    pub spr: usize,
    pub angle: f32
}

impl MotorParams {
    pub fn new(step_pin: u8, dir_pin: u8, sleep_pin: u8, spr: usize, angle: f32) -> Self {
        Self {
            step_pin,
            dir_pin,
            sleep_pin,
            spr,
            angle
        }
    }
}

impl Default for MotorParams {
    fn default() -> Self {
        Self {
            step_pin: consts::STEP_PIN,
            dir_pin: consts::DIR_PIN,
            sleep_pin: consts::SLEEP_PIN,
            spr: consts::SPR,
            angle: consts::ANGLE
        }
    }
}

enum Pulse {
    High,
    StepsUntilPulse(u32),
}

#[derive(PartialEq, Copy, Clone)]
pub struct TurnSteps(pub u32);

impl TurnSteps {
    fn count(&self) -> u32 {
        self.0
    }
}

#[derive(PartialEq, Copy, Clone)]
pub struct Ticks(pub u32);

impl Ticks {
    fn count(&self) -> u32 {
        self.0
    }
}

trait NumberTraits {
    fn ticks(self) -> Ticks;
    fn turn_steps(self) -> TurnSteps;
}

impl NumberTraits for u32 {
    fn ticks(self) -> Ticks {
        Ticks(self)
    }

    fn turn_steps(self) -> TurnSteps {
       TurnSteps(self) 
    }
}

#[derive(Debug)]
pub enum StateError { 
    AlreadyTurning,
    NeedMoreTicksPerStep,
}

#[derive(PartialEq)]
enum State {
    Idle,
    Moving {
        turn_steps: TurnSteps,
        ticks_per_step: Ticks,
    },
}

pub struct Motor {
    pub params: MotorParams,
    step_pin_ot: Output<'static, AnyPin>,
    dir_pin_ot: Output<'static, AnyPin>,
    sleep_pin_ot: Output<'static, AnyPin>,
    pub steps: usize,
    pulse: Pulse,
    state: State,
    last: Option<Instant>,
    hanger: Duration,
}

impl Motor {
    pub fn new(params: MotorParams, step_pin_ow: impl Pin, dir_pin_ow: impl Pin, sleep_pin_ow: impl Pin) -> Self {
        if step_pin_ow.pin() != params.step_pin
           || dir_pin_ow.pin() != params.dir_pin
           || sleep_pin_ow.pin() != params.sleep_pin
        {
            panic!("Passed wrong pins!");
        }

        let step_pin_ow = step_pin_ow.degrade();
        let dir_pin_ow = dir_pin_ow.degrade();
        let sleep_pin_ow =sleep_pin_ow.degrade();

        let step_ot = Output::new(step_pin_ow, Level::Low);
        let dir_ot = Output::new(dir_pin_ow, Level::Low);
        let sleep_ot = Output::new(sleep_pin_ow, Level::High);

        Motor {
            params,
            step_pin_ot: step_ot,
            dir_pin_ot: dir_ot,
            sleep_pin_ot: sleep_ot,
            steps: 0,
            pulse: Pulse::StepsUntilPulse(0),
            state: State::Idle,
            last: None,
            hanger: Duration::from_micros(0),
        }
    }

    pub async fn toggle_dir(&mut self) -> Result<(), StateError> {
        if self.state != State::Idle {
            return Err(StateError::AlreadyTurning)
        } 
        self.dir_pin_ot.toggle();
        Ok(())
    }

    pub async fn start_turning(&mut self, turn_steps: TurnSteps, ticks_per_step: Ticks) -> Result<(), StateError> {
        if self.state != State::Idle {
            self.sleep_pin_ot.set_low();
            return Err(StateError::AlreadyTurning)
        }

        if ticks_per_step.count() < 1 {
            self.sleep_pin_ot.set_low();
            return Err(StateError::NeedMoreTicksPerStep);
        }

        self.state = State::Moving { 
            turn_steps, 
            ticks_per_step: Ticks(ticks_per_step.count() - 1),
        };
        self.pulse = Pulse::StepsUntilPulse(0);

        if self.sleep_pin_ot.is_set_low() {
            self.sleep_pin_ot.set_high();
        }

        loop {
            match self.state {
                State::Idle => {
                    self.sleep_pin_ot.set_low();
                    return Ok(())
                },
                State::Moving { 
                    turn_steps, 
                    ticks_per_step 
                } => {
                    self.pulse = match self.pulse {
                        Pulse::High => {
                            self.step_pin_ot.set_low();
                            Pulse::StepsUntilPulse(ticks_per_step.count())
                        }
                        Pulse::StepsUntilPulse(0) => {
                            if turn_steps.count() == 0 {
                                self.state = State::Idle;
                                self.sleep_pin_ot.set_low();
                                return Ok(())
                            }
                            self.state = State::Moving { 
                                turn_steps: TurnSteps(turn_steps.count() - 1),
                                ticks_per_step,
                            };
                            
                            self.last = Some(Instant::now());
                            self.hanger = Duration::from_micros(0);

                            self.step_pin_ot.set_high();
                            Pulse::High
                        }
                        Pulse::StepsUntilPulse(n) => {
                            let (sleep, hanger) = self.calc_sleep();
                            Timer::after(sleep).await;

                            self.hanger = hanger;
                            self.last = Some(Instant::now());

                            rlog!(&"da");

                            Pulse::StepsUntilPulse(n - 1)
                        },
                    };
                }
            }
        }
     }

    fn calc_sleep(&self) -> (Duration, Duration) {
        if let Some(last) = self.last {
            let now = Instant::now();
            let lateness = now.saturating_duration_since(last);
            return Self::positive_or_zero(consts::MAX_SPEED as i64 - lateness.as_micros() as i64 - self.hanger.as_micros() as i64);
        } else {
            return (Duration::from_micros(consts::MAX_SPEED), Duration::from_micros(0));
        }
    }

    fn positive_or_zero(duration: i64) -> (Duration, Duration) {
        if duration >= 0 {
            return (Duration::from_micros(duration as u64), Duration::from_micros(0))
        } else {
            return (Duration::from_micros(0), Duration::from_micros((duration * -1) as u64));
        }
    }
}

#[embassy_executor::task]
pub async fn global_turn(mut stepper: Motor, turn_steps: TurnSteps, ticks: Ticks) -> () {
    if let Err(e) = stepper.start_turning(turn_steps, ticks).await {
        panic!("State error {:?}", e);
    } else {
        ()
    }
} 
