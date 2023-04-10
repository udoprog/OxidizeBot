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
    () => {
        crate::command::Respond::Empty
    };

    ($m:expr) => {
        crate::command::Respond::Message(std::borrow::Cow::Borrowed($m))
    };

    ($($t:tt)*) => {
        crate::command::Respond::Message(std::borrow::Cow::Owned(format!($($t)*)))
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
