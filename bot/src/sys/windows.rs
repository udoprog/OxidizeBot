use std::future::Future;
use std::io;
use std::path::Path;
use std::ptr;
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::Local;
use tokio::sync::{mpsc, Notify};
use winapi::um::shellapi::ShellExecuteW;
use winapi::um::winuser::SW_SHOW;

use crate::sys::Notification;

mod convert;
mod registry;
mod window;

const ICON: &[u8] = include_bytes!("../../res/icon.ico");
const ICON_ERROR: &[u8] = include_bytes!("../../res/icon-error.ico");

#[derive(Debug)]
pub(crate) enum Event {
    Shutdown,
    Cleared,
    Errored(String),
    Notification(Notification),
}

pub(crate) struct SystemNotify {
    shutdown: Notify,
    restart: Arc<Notify>,
}

impl Default for SystemNotify {
    #[inline]
    fn default() -> Self {
        Self {
            shutdown: Notify::default(),
            restart: Arc::new(Notify::default()),
        }
    }
}

#[derive(Clone)]
pub(crate) struct System {
    notify: Arc<SystemNotify>,
    events: mpsc::UnboundedSender<Event>,
}

impl System {
    /// Wait for system shutdown signal.
    pub(crate) async fn wait_for_shutdown(&self) {
        self.notify.shutdown.notified().await;
    }

    /// Wait for system restart signal.
    pub(crate) async fn wait_for_restart(&self) {
        self.notify.restart.notified().await;
    }

    /// Get restart notification helper.
    pub(crate) fn restart(&self) -> &Arc<Notify> {
        &self.notify.restart
    }

    /// Clear the current state.
    pub(crate) fn clear(&self) {
        if let Err(e) = self.events.send(Event::Cleared) {
            tracing::error!("Failed to send clear: {}", e);
        }
    }

    /// Set an error.
    pub(crate) fn error(&self, error: String) {
        if let Err(e) = self.events.send(Event::Errored(error)) {
            tracing::error!("Failed to send clear: {}", e);
        }
    }

    /// Send the given notification.
    pub(crate) fn notification(&self, n: Notification) {
        if let Err(e) = self.events.send(Event::Notification(n)) {
            tracing::error!("Failed to send notification: {}", e);
        }
    }

    /// Join the current thread.
    pub(crate) fn shutdown(&self) {
        if let Err(e) = self.events.send(Event::Shutdown) {
            tracing::error!("Failed to send shutdown: {}", e);
        }
    }

    /// Entry for automatic startup.
    fn run_registry_entry() -> Result<String> {
        let exe = std::env::current_exe()?;
        let exe = exe.to_str().context("bad executable string")?;
        Ok(format!("\"{}\" --silent", exe))
    }

    /// If the program is installed to run at startup.
    pub(crate) fn is_installed(&self) -> Result<bool> {
        let key = self::registry::RegistryKey::current_user(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        )?;

        let path = match key.get("OxidizeBot")? {
            Some(path) => path,
            None => return Ok(false),
        };

        Ok(Self::run_registry_entry()?.as_str() == path)
    }

    pub(crate) fn install(&self) -> Result<()> {
        let key = self::registry::RegistryKey::current_user(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        )?;

        key.set("OxidizeBot", Self::run_registry_entry()?)?;
        Ok(())
    }

    pub(crate) fn uninstall(&self) -> Result<()> {
        let key = self::registry::RegistryKey::current_user(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        )?;

        key.delete("OxidizeBot")?;
        Ok(())
    }
}

/// Open the given directory.
fn open_dir(path: &Path) -> io::Result<bool> {
    use self::convert::ToWide as _;

    let path = path.to_wide_null();
    let operation = "open".to_wide_null();

    let result = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            operation.as_ptr(),
            path.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOW,
        )
    };

    Ok(result as usize > 32)
}

pub(crate) fn setup<'a>(
    root: &'a Path,
    log_file: &'a Path,
) -> Result<(System, impl Future<Output = ()> + 'a)> {
    let root = root.to_owned();
    let log_file = log_file.to_owned();

    let notify = Arc::new(SystemNotify::default());
    let notify1 = notify.clone();

    let (events, events_rx) = mpsc::unbounded_channel::<Event>();

    let window_loop = async move {
        fn setup_menu(window: &mut window::Window) -> Result<()> {
            window.add_menu_entry(0, &format!("OxidizeBot {}", crate::VERSION), true)?;
            window.add_menu_separator(1)?;
            window.add_menu_entry(2, "Log File ...", false)?;
            window.add_menu_entry(3, "Directory ...", false)?;
            window.add_menu_entry(4, "Restart", false)?;
            window.add_menu_separator(5)?;
            window.add_menu_entry(6, "Exit", false)?;
            Ok(())
        }

        async fn window_loop(
            root: &Path,
            log_file: &Path,
            mut events_rx: mpsc::UnboundedReceiver<Event>,
            notify: &SystemNotify,
            window: &mut window::Window,
        ) -> Result<()> {
            window.set_icon_from_buffer(ICON, 128, 128)?;
            setup_menu(window).context("Setting up menu")?;

            let mut on_click = Vec::new();

            loop {
                tokio::select! {
                    Some(event) = events_rx.recv() => {
                        tracing::trace!("Event: {:?}", event);

                        match event {
                            Event::Cleared => {
                                window.set_icon_from_buffer(ICON, 128, 128)?;
                            }
                            Event::Errored(message) => {
                                let message = message.to_string();
                                window.set_tooltip(&message)?;
                                window.set_icon_from_buffer(ICON_ERROR, 128, 128)?;
                            }
                            Event::Notification(mut n) => {
                                on_click.push(n.on_click.take());
                                window.send_notification(n)?;
                            }
                            Event::Shutdown => {
                                break;
                            }
                        }
                    }
                    e = window.tick() => {
                        match e {
                            window::Event::MenuClicked(idx) => match idx {
                                0 => {
                                    webbrowser::open(web::URL)?;
                                }
                                2 => {
                                    let date = Local::now().date_naive();
                                    let log_file = log_file.with_extension(format!("log.{date}"));
                                    let _ = open_dir(&log_file)?;
                                }
                                3 => {
                                    let _ = open_dir(root)?;
                                }
                                4 => {
                                    notify.restart.notify_one();
                                }
                                6 => {
                                    notify.shutdown.notify_one();
                                }
                                _ => (),
                            },
                            window::Event::Shutdown => {
                                notify.shutdown.notify_one();
                            }
                            window::Event::BalloonClicked => {
                                if let Some(Some(mut cb)) = on_click.pop() {
                                    cb()?;
                                }
                            }
                            window::Event::BalloonTimeout => {
                                let _ = on_click.pop();
                            }
                        }
                    }
                }
            }

            Ok(())
        }

        let mut window = match window::Window::new(String::from("OxidizeBot")).await {
            Ok(window) => window,
            Err(error) => {
                common::log_error!(error, "Failed to setup window");
                return;
            }
        };

        if let Err(error) = window_loop(&root, &log_file, events_rx, &notify, &mut window).await {
            common::log_error!(error, "Window loop failed");
        }

        window.join();
    };

    let system = System {
        notify: notify1,
        events,
    };

    Ok((system, window_loop))
}
