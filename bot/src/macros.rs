/// Helper macro to log an error and all it's causes.
#[macro_export]
macro_rules! log_err {
    ($e:expr, $fmt:expr $(, $($arg:tt)*)?) => {{
        let e = anyhow::Error::from($e);

        log::error!("{what}: {error}", what = format!($fmt $(, $($arg)*)*), error = e);

        #[cfg(feature = "nightly")]
        {
            log::error!("{:?}", e.backtrace());
        }

        for e in e.chain().skip(1) {
            log::error!("caused by: {}", e);

            #[cfg(feature = "nightly")]
            {
                log::error!("{:?}", e.backtrace());
            }
        }
    }};
}

/// Helper macro to handle the result of polling an infinite stream.
#[macro_export]
macro_rules! try_infinite {
    ($expr:expr) => {
        match $expr {
            Err(e) => return Err(e.into()),
            Ok(a) => match a {
                futures01::Async::NotReady => None,
                futures01::Async::Ready(None) => anyhow::bail!("stream ended unexpectedly"),
                futures01::Async::Ready(Some(v)) => Some(v),
            },
        }
    };
}

/// Helper macro to handle the result of polling an infinite stream that can error with a unit.
#[macro_export]
macro_rules! try_infinite_empty {
    ($expr:expr) => {
        try_infinite!($expr.map_err(|()| anyhow::anyhow!("stream unexpectedly errored")))
    };
}

/// Handle a context argument result.
///
/// Returns Ok(()) in case a context argument result is `None`.
#[macro_export]
macro_rules! ctx_try {
    ($expr:expr) => {
        match $expr {
            Some(value) => value,
            None => return Ok(()),
        }
    };
}
