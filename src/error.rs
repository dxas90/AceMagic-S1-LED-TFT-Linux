use std::fmt;

#[derive(Debug)]
pub enum Error {
    Hid(hidapi::HidError),
    Serial(serialport::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    Toml(toml::de::Error),
    Config(String),
    Render(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hid(e) => write!(f, "HID error: {e}"),
            Self::Serial(e) => write!(f, "Serial error: {e}"),
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Json(e) => write!(f, "JSON error: {e}"),
            Self::Toml(e) => write!(f, "TOML error: {e}"),
            Self::Config(s) => write!(f, "Config error: {s}"),
            Self::Render(s) => write!(f, "Render error: {s}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<hidapi::HidError> for Error {
    fn from(e: hidapi::HidError) -> Self {
        Self::Hid(e)
    }
}
impl From<serialport::Error> for Error {
    fn from(e: serialport::Error) -> Self {
        Self::Serial(e)
    }
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Self::Toml(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
