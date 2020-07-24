use anyhow::Error;
use std::fmt;
use std::time::Duration;

#[cfg(not(target_os = "windows"))]
#[path = "noop.rs"]
mod imp;
#[cfg(target_os = "windows")]
#[path = "windows/mod.rs"]
mod imp;

#[derive(Debug, Clone, Copy)]
pub enum NotificationIcon {
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
pub struct Notification {
    pub message: String,
    pub title: Option<String>,
    pub icon: NotificationIcon,
    pub timeout: Option<Duration>,
    pub on_click: Option<Callback>,
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
    pub fn new<M>(message: M) -> Self
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
    pub fn title<T>(self, title: T) -> Self
    where
        T: AsRef<str>,
    {
        Self {
            title: Some(title.as_ref().to_string()),
            ..self
        }
    }

    /// Set the notification timeout.
    pub fn timeout(self, timeout: Duration) -> Self {
        Self {
            timeout: Some(timeout),
            ..self
        }
    }

    /// What should happen if we click the notification.
    pub fn on_click<F>(self, on_click: F) -> Self
    where
        F: FnMut() -> Result<(), Error> + Send + 'static,
    {
        Self {
            on_click: Some(Box::new(on_click)),
            ..self
        }
    }

    /// Set the notification icon.
    pub fn icon(self, icon: NotificationIcon) -> Self {
        Self { icon, ..self }
    }
}

pub use self::imp::{setup, System};
