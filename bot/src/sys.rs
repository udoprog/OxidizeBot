use anyhow::Error;
use std::fmt;
use std::time::Duration;

#[cfg(not(target_os = "windows"))]
mod noop;
#[cfg(not(target_os = "windows"))]
use noop as imp;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as imp;

#[derive(Debug, Clone, Copy)]
pub(crate) enum NotificationIcon {
    Info,
    Warning,
    Error,
}

// Windows-specific implementation details.
#[cfg(target_os = "windows")]
impl NotificationIcon {
    /// Convert into a flag.
    fn into_flags(self) -> winapi::shared::minwindef::DWORD {
        use self::NotificationIcon::*;
        use winapi::um::shellapi;

        match self {
            Info => shellapi::NIIF_INFO,
            Error => shellapi::NIIF_ERROR,
            Warning => shellapi::NIIF_WARNING,
        }
    }
}

type Callback = Box<dyn FnMut() -> Result<(), Error> + Send + 'static>;

/// A single notification.
pub(crate) struct Notification {
    pub(crate) message: String,
    pub(crate) title: Option<String>,
    pub(crate) icon: NotificationIcon,
    pub(crate) timeout: Option<Duration>,
    pub(crate) on_click: Option<Callback>,
}

impl fmt::Debug for Notification {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Notification")
            .field("message", &self.message)
            .field("title", &self.title)
            .field("icon", &self.icon)
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl Notification {
    /// Create a new notification.
    pub(crate) fn new<M>(message: M) -> Self
    where
        M: AsRef<str>,
    {
        Self {
            message: message.as_ref().to_string(),
            title: None,
            icon: NotificationIcon::Info,
            timeout: Some(Duration::from_secs(1)),
            on_click: None,
        }
    }

    /// Set the message for the notification.
    pub(crate) fn title<T>(self, title: T) -> Self
    where
        T: AsRef<str>,
    {
        Self {
            title: Some(title.as_ref().to_string()),
            ..self
        }
    }

    /// What should happen if we click the notification.
    #[cfg_attr(not(windows), allow(unused))]
    pub(crate) fn on_click<F>(self, on_click: F) -> Self
    where
        F: FnMut() -> Result<(), Error> + Send + 'static,
    {
        Self {
            on_click: Some(Box::new(on_click)),
            ..self
        }
    }

    /// Set the notification icon.
    pub(crate) fn icon(self, icon: NotificationIcon) -> Self {
        Self { icon, ..self }
    }
}

pub(crate) use self::imp::{setup, System};
