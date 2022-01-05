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
        $crate::log_base!(warn, $e, $fmt $(, $($arg)*)*)
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

/// Helper macro to generate a respond error.
macro_rules! respond_err {
    ($m:expr) => {
        crate::command::Respond(std::borrow::Cow::Borrowed($m))
    };

    ($($t:tt)*) => {
        crate::command::Respond(std::borrow::Cow::Owned(format!($($t)*)))
    };
}

/// Helper macro to bail with a respond error.
macro_rules! respond_bail {
    ($($t:tt)*) => {
        return Err(respond_err!($($t)*).into())
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
    ($ctx:expr, $argument:expr) => {{
        $ctx.respond($argument).await;
    }};

    ($ctx:expr, $($t:tt)*) => {{
        $ctx.respond(&format!($($t)*)).await;
    }};
}

/// Backoff and retry until the given action has been successfully executed.
#[macro_export]
macro_rules! retry_until_ok {
    ($id:expr, { $($f:tt)* }) => {
        async {
            let mut backoff = $crate::backoff::Exponential::new(std::time::Duration::from_secs(2));

            loop {
                log::info!("{}", $id);
                let res: anyhow::Result<_> = async { $($f)* }.await;

                match res {
                    Ok(output) => break output,
                    Err(e) => {
                        let duration = backoff.next();
                        log_warn!(e, "{} failed, trying again in {:?}", $id, duration);
                        tokio::time::sleep(duration).await;
                    }
                }
            }
        }
    };
}
