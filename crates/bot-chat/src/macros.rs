
/// Generate a [`RespondErr`][crate::RespondErr].
#[macro_export]
macro_rules! respond_err {
    () => {
        $crate::RespondErr::Empty
    };

    ($m:expr) => {
        $crate::RespondErr::Message(std::borrow::Cow::Borrowed($m))
    };

    ($($t:tt)*) => {
        $crate::RespondErr::Message(std::borrow::Cow::Owned(format!($($t)*)))
    };
}

/// Helper macro to bail with a respond error.
///
/// Bail from the current function with the given response.
#[macro_export]
macro_rules! respond_bail {
    ($($t:tt)*) => {
        return Err($crate::respond_err!($($t)*).into())
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
