use core::str::FromStr;

pub enum Color {
    Red,
    Green,
    Blue,
    Reset,
}

#[derive(Debug)]
pub struct ParseColorError {}

impl Color {
    pub fn make(&self) -> &'static str {
        match self {
            Self::Red => "\x1b[31m",
            Self::Green => "\x1b[32m",
            Self::Blue => "\x1b[34m",
            Self::Reset => "\x1b[0m",
        }
    }

    pub fn to_log_severity(&self) -> &'static str {
        match self {
            Self::Red => "ERROR",
            Self::Green => "LOG",
            Self::Blue => "WARNING",
            _ => unreachable!(),
        }
    }
}

impl FromStr for Color {
    type Err = ParseColorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "\x1b[31m" => Ok(Self::Red),
            "\x1b[32m" => Ok(Self::Green),
            "\x1b[34m" => Ok(Self::Blue),
            "\x1b[0m" => Ok(Self::Reset),
            _ => Err(ParseColorError {  })
        }
    }
}
