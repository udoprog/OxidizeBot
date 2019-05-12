/// Helper macro to log an error and all it's causes.
#[macro_export]
macro_rules! log_err {
    ($e:expr, $fmt:expr $(, $($arg:tt)*)?) => {{
        let e = failure::Error::from($e);

        log::error!("{what}: {error}", what = format!($fmt $(, $($arg)*)*), error = e);
        log::error!("backtrace: {}", e.backtrace());

        for cause in e.iter_causes() {
            log::error!("caused by: {}", cause);
            log::error!("backtrace: {}", e.backtrace());
        }
    }};
}
