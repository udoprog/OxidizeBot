use std::time::Duration;

#[cfg(target_os = "windows")]
#[path = "sys/windows.rs"]
mod imp;
#[cfg(not(target_os = "windows"))]
#[path = "sys/noop.rs"]
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

/// A single notification.
#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub title: Option<String>,
    pub icon: NotificationIcon,
    pub timeout: Option<Duration>,
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

    /// Set the notification icon.
    pub fn icon(self, icon: NotificationIcon) -> Self {
        Self { icon, ..self }
    }
}

pub use self::imp::{setup, System};
