use std::fmt;

#[derive(Debug)]
pub enum Error {
    Init(init_tracing_opentelemetry::Error),
    AlreadyInitialized,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Init(err) => write!(f, "{err}"),
            Self::AlreadyInitialized => write!(f, "logger already initialized"),
        }
    }
}

impl std::error::Error for Error {}

impl From<init_tracing_opentelemetry::Error> for Error {
    fn from(value: init_tracing_opentelemetry::Error) -> Self {
        Self::Init(value)
    }
}
