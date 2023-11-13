#[macro_export]
macro_rules! rlog {
    ($string:expr) => {
        log($string, Color::Green)
    };
}

#[macro_export]
macro_rules! rwarn {
    ($string:expr) => {
        log($string, Color::Blue)
    };
}

#[macro_export]
macro_rules! rerror {
    ($string:expr) => {
        log($string, Color::Red)
    };
}
