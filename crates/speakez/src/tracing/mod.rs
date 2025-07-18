#[cfg(feature = "tracing")]
macro_rules! info {
    ($($arg:tt)*) => {
        tracing::info!($($arg)*);
    };
}

#[cfg(not(feature = "tracing"))]
macro_rules! info {
    ($($arg:tt)*) => {};
}

#[cfg(feature = "tracing")]
macro_rules! error {
    ($($arg:tt)*) => {
        tracing::error!($($arg)*);
    };
}

#[cfg(not(feature = "tracing"))]
macro_rules! error {
    ($($arg:tt)*) => {};
}

#[cfg(feature = "tracing")]
macro_rules! warn_level {
    ($($arg:tt)*) => {
        tracing::warn!($($arg)*);
    };
}

#[cfg(not(feature = "tracing"))]
macro_rules! warn_level {
    ($($arg:tt)*) => {};
}

#[cfg(feature = "tracing")]
macro_rules! debug {
    ($($arg:tt)*) => {
        tracing::debug!($($arg)*);
    };
}

#[cfg(not(feature = "tracing"))]
macro_rules! debug {
    ($($arg:tt)*) => {};
}

pub(crate) use debug;
pub(crate) use error;
pub(crate) use info;
pub(crate) use warn_level as warn;
