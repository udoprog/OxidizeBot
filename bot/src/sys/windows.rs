use std::collections::VecDeque;
use std::io;
use std::path::Path;
use std::ptr;
use std::sync::Arc;
use std::thread;

use anyhow::{anyhow, bail, Context as _, Error};
use chrono::Local;
use parking_lot::Mutex;
use tokio::sync::{mpsc, Notify, Semaphore};
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
    Cleared,
    Errored(String),
    Notification(Notification),
}

pub(crate) struct SystemNotify {
    shutdown: Semaphore,
    restart: Arc<Notify>,
}

impl Default for SystemNotify {
    #[inline]
    fn default() -> Self {
        Self {
            shutdown: Semaphore::new(0),
            restart: Arc::new(Notify::default()),
        }
    }
}

#[derive(Clone)]
pub(crate) struct System {
    thread: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
    notify: Arc<SystemNotify>,
    events: mpsc::UnboundedSender<Event>,
}

impl System {
    /// Wait for system shutdown signal.
    pub(crate) async fn wait_for_shutdown(&self) {
        let _ = self.notify.shutdown.acquire().await;
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
    pub(crate) fn join(&self) -> Result<(), Error> {
        self.notify.shutdown.add_permits(1);

        if let Some(thread) = self.thread.lock().take() {
            if thread.join().is_err() {
                bail!("background thread panicked");
            }
        }

        Ok(())
    }

    /// Entry for automatic startup.
    fn run_registry_entry() -> Result<String, Error> {
        let exe = std::env::current_exe()?;

        let exe = exe
            .to_str()
            .ok_or_else(|| anyhow!("bad executable string"))?;

        Ok(format!("\"{}\" --silent", exe))
    }

    /// If the program is installed to run at startup.
    pub(crate) fn is_installed(&self) -> Result<bool, Error> {
        let key = self::registry::RegistryKey::current_user(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        )?;

        let path = match key.get("OxidizeBot")? {
            Some(path) => path,
            None => return Ok(false),
        };

        Ok(Self::run_registry_entry()?.as_str() == path)
    }

    pub(crate) fn install(&self) -> Result<(), Error> {
        let key = self::registry::RegistryKey::current_user(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        )?;

        key.set("OxidizeBot", Self::run_registry_entry()?)?;
        Ok(())
    }

    pub(crate) fn uninstall(&self) -> Result<(), Error> {
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

pub(crate) fn setup(root: &Path, log_file: &Path) -> Result<System, Error> {
    let root = root.to_owned();
    let log_file = log_file.to_owned();

    let notify = Arc::new(SystemNotify::default());
    let notify1 = notify.clone();

    let (events, mut events_rx) = mpsc::unbounded_channel::<Event>();

    let window_loop = async move {
        let mut window = window::Window::new(String::from("OxidizeBot")).await?;

        window.set_icon_from_buffer(ICON, 128, 128)?;

        window.add_menu_entry(0, &format!("OxidizeBot {}", crate::VERSION), true)?;
        window.add_menu_separator(1)?;
        window.add_menu_entry(2, "Log File ...", false)?;
        window.add_menu_entry(3, "Directory ...", false)?;
        window.add_menu_entry(4, "Restart", false)?;
        window.add_menu_separator(5)?;
        window.add_menu_entry(6, "Exit", false)?;

        let mut notification_on_click = VecDeque::new();

        loop {
            tokio::select! {
                _ = notify.shutdown.acquire() => {
                    break;
                }
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
                            notification_on_click.push_back(n.on_click.take());
                            window.send_notification(n)
                            .context("sending notification")?;
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
                                let _ = open_dir(&root)?;
                            }
                            4 => {
                                notify.restart.notify_one();
                            }
                            6 => {
                                break;
                            }
                            _ => (),
                        },
                        window::Event::Shutdown => {
                            break;
                        }
                        window::Event::BalloonClicked => {
                            if let Some(Some(mut cb)) = notification_on_click.pop_front() {
                                cb()?;
                            }
                        }
                        window::Event::BalloonTimeout => {
                            let _ = notification_on_click.pop_front();
                        }
                    }
                }
            }
        }

        window.quit();
        notify.shutdown.add_permits(1);
        Ok::<_, Error>(())
    };

    let thread = thread::spawn(move || match futures_executor::block_on(window_loop) {
        Ok(()) => (),
        Err(e) => {
            common::log_error!(e, "Windows system tray errored");
        }
    });

    let system = System {
        thread: Arc::new(Mutex::new(Some(thread))),
        notify: notify1,
        events,
    };

    Ok(system)
}
