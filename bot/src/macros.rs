/// Helper macro to log an error and all its causes.
#[macro_export]
macro_rules! log_error {
    ($e:expr, $fmt:expr $(, $($arg:tt)*)?) => {
        $crate::log_base!(error, $e, $fmt $(, $($arg)*)*)
    };
}

/// Helper macro to log a warning and all its causes.
#[macro_export]
macro_rules! log_warn {
    ($e:expr, $fmt:expr $(, $($arg:tt)*)?) => {
        $crate::log_base!(error, $e, $fmt $(, $($arg)*)*)
    };
}

#[macro_export]
macro_rules! log_base {
    ($level:tt, $e:expr, $fmt:expr $(, $($arg:tt)*)?) => {{
        let e = anyhow::Error::from($e);

        log::$level!($fmt $(, $($arg)*)*);

        for e in e.chain() {
            #[cfg(not(backtrace))]
            {
                log::$level!("caused by: {}", e);
            }

            #[cfg(backtrace)]
            {
                if let Some(bt) = e.backtrace() {
                    log::$level!("caused by: {}\n{}", e, bt);
                } else {
                    log::$level!("caused by: {}", e);
                }
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

/// Helper macro to handle sending a response.
///
/// # Examples
///
/// ```ignore
/// let name = "Joseph";
/// oxidize::respond!(ctx, "Hello {}", name);
/// ```
#[macro_export]
macro_rules! respond {
    ($ctx:expr, $($t:tt)*) => {{
        $ctx.respond(&format!($($t)*));
    }}
}
