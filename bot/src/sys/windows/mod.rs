use crate::prelude::*;
use crate::sys::Notification;
use crate::web;
use anyhow::{anyhow, bail, Context as _, Error};
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::io;
use std::path::Path;
use std::ptr;
use std::sync::Arc;
use std::thread;
use tokio::sync::broadcast;
use winapi::um::shellapi::ShellExecuteW;
use winapi::um::winuser::SW_SHOW;

mod convert;
mod registry;
mod window;

const ICON: &[u8] = include_bytes!("../../../res/icon.ico");
const ICON_ERROR: &[u8] = include_bytes!("../../../res/icon-error.ico");

#[derive(Debug)]
pub enum Event {
    Cleared,
    Errored(String),
    Notification(Notification),
}

#[derive(Clone)]
pub struct System {
    thread: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
    shutdown: broadcast::Sender<()>,
    restart: broadcast::Sender<()>,
    events: mpsc::UnboundedSender<Event>,
}

impl System {
    /// Wait for system shutdown signal.
    pub async fn wait_for_shutdown(&self) {
        let _ = self.shutdown.subscribe().recv().await;
    }

    /// Wait for system restart signal.
    pub async fn wait_for_restart(&self) {
        let _ = self.restart.subscribe().recv().await;
    }

    /// Clear the current state.
    pub fn clear(&self) {
        if let Err(e) = self.events.send(Event::Cleared) {
            log::error!("failed to send clear: {}", e);
        }
    }

    /// Set an error.
    pub fn error(&self, error: String) {
        if let Err(e) = self.events.send(Event::Errored(error)) {
            log::error!("failed to send clear: {}", e);
        }
    }

    /// Send the given notification.
    pub fn notification(&self, n: Notification) {
        if let Err(e) = self.events.send(Event::Notification(n)) {
            log::error!("failed to send notification: {}", e);
        }
    }

    /// Join the current thread.
    pub fn join(&self) -> Result<(), Error> {
        let _ = self.shutdown.send(());

        if let Some(thread) = self.thread.lock().take() {
            if thread.join().is_err() {
                bail!("thread panicked");
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
    pub fn is_installed(&self) -> Result<bool, Error> {
        let key = self::registry::RegistryKey::current_user(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        )?;

        let path = match key.get("OxidizeBot")? {
            Some(path) => path,
            None => return Ok(false),
        };

        Ok(Self::run_registry_entry()?.as_str() == path)
    }

    pub fn install(&self) -> Result<(), Error> {
        let key = self::registry::RegistryKey::current_user(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        )?;

        key.set("OxidizeBot", &Self::run_registry_entry()?)?;
        Ok(())
    }

    pub fn uninstall(&self) -> Result<(), Error> {
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

pub fn setup(root: &Path, log_file: &Path) -> Result<System, Error> {
    let root = root.to_owned();
    let log_file = log_file.to_owned();

    // all senders to notify when we are requesting a restart.
    let (restart, _) = broadcast::channel(1);
    let restart1 = restart.clone();

    // all senders to notify when we are shutting down.
    let (shutdown, mut shutdown_rx) = broadcast::channel(1);
    let shutdown1 = shutdown.clone();

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
                _ = shutdown_rx.recv() => {
                    window.quit();
                }
                Some(event) = events_rx.recv() => {
                    log::trace!("Event: {:?}", event);

                    match event {
                        Event::Cleared => {
                            window.set_icon_from_buffer(ICON, 128, 128)?;
                        }
                        Event::Errored(message) => {
                            let message = format!("{}", message);
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
                                let _ = webbrowser::open(web::URL)?;
                            }
                            2 => {
                                let _ = open_dir(&log_file)?;
                            }
                            3 => {
                                let _ = open_dir(&root)?;
                            }
                            4 => {
                                let _ = restart1.send(());
                            }
                            6 => {
                                window.quit();
                                let _ = shutdown1.send(());
                            }
                            _ => (),
                        },
                        window::Event::Shutdown => {
                            break;
                        }
                        window::Event::BalloonClicked => {
                            if let Some(Some(mut cb)) = notification_on_click.pop_front() {
                                let _ = cb()?;
                            }
                        }
                        window::Event::BalloonTimeout => {
                            let _ = notification_on_click.pop_front();
                        }
                    }
                }
            }
        }

        let _ = shutdown1.send(());
        Ok::<_, Error>(())
    };

    let thread = thread::spawn(move || match futures_executor::block_on(window_loop) {
        Ok(()) => (),
        Err(e) => {
            log_error!(e, "Windows system tray errored");
        }
    });

    let system = System {
        thread: Arc::new(Mutex::new(Some(thread))),
        shutdown,
        restart,
        events,
    };

    Ok(system)
}
