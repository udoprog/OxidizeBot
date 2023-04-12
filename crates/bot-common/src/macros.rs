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

        ::tracing::$level!($fmt $(, $($arg)*)*);

        for e in e.chain() {
            ::tracing::$level!("Caused by: {}", e);
        }
    }};
}

/// Backoff and retry until the given action has been successfully executed.
#[macro_export]
macro_rules! retry_until_ok {
    ($id:expr, { $($f:tt)* }) => {
        'output: {
            let mut backoff = $crate::backoff::Exponential::new(::std::time::Duration::from_secs(2));

            loop {
                ::tracing::info!("{}", $id);
                let res: anyhow::Result<_> = async { $($f)* }.await;

                match res {
                    Ok(output) => break 'output output,
                    Err(e) => {
                        let duration = backoff.failed();
                        $crate::log_warn!(e, "{} failed, trying again in {:?}", $id, duration);
                        ::tokio::time::sleep(duration).await;
                    }
                }
            }
        }
    };
}
