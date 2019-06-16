use crate::{prelude::*, sys::Notification, web};
use failure::{bail, format_err, Error};
use parking_lot::Mutex;
use std::{
    io,
    path::Path,
    ptr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};
use winapi::um::{shellapi, winuser::SW_SHOW};

#[path = "windows/convert.rs"]
mod convert;
#[path = "windows/registry.rs"]
mod registry;
#[path = "windows/window.rs"]
mod window;

const ICON: &[u8] = include_bytes!("../../res/icon.ico");
const ICON_ERROR: &[u8] = include_bytes!("../../res/icon-error.ico");

pub enum Event {
    Cleared,
    Errored(String),
    Notification(Notification),
}

#[derive(Clone)]
pub struct System {
    thread: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
    shutdown_senders: Arc<Mutex<Vec<oneshot::Sender<()>>>>,
    restart_senders: Arc<Mutex<Vec<oneshot::Sender<()>>>>,
    events: mpsc::UnboundedSender<Event>,
    stopped: Arc<AtomicBool>,
}

impl System {
    /// Wait for system shutdown signal.
    pub async fn wait_for_shutdown(&self) -> Result<(), oneshot::Canceled> {
        let (tx, rx) = oneshot::channel();
        self.shutdown_senders.lock().push(tx);
        rx.await?;
        self.stopped.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Wait for system restart signal.
    pub async fn wait_for_restart(&self) -> Result<(), oneshot::Canceled> {
        let (tx, rx) = oneshot::channel();
        self.restart_senders.lock().push(tx);
        rx.await?;
        Ok(())
    }

    /// Clear the current state.
    pub fn clear(&self) {
        if let Err(e) = self.events.unbounded_send(Event::Cleared) {
            log::error!("failed to send clear: {}", e);
        }
    }

    /// Set an error.
    pub fn error(&self, error: String) {
        if let Err(e) = self.events.unbounded_send(Event::Errored(error)) {
            log::error!("failed to send clear: {}", e);
        }
    }

    /// Send the given notification.
    pub fn notification(&self, n: Notification) {
        if let Err(e) = self.events.unbounded_send(Event::Notification(n)) {
            log::error!("failed to send notification: {}", e);
        }
    }

    /// Test if system is running.
    pub fn is_running(&self) -> bool {
        // NB: this side effect is a bit unintuitive, but we know that is_running will only be called by the main loop
        // when the bot core has been shutdown, and any futures that have called wait_for_shutdown have been dropped.
        self.shutdown_senders.lock().clear();
        !self.stopped.load(Ordering::SeqCst)
    }

    /// Join the current thread.
    pub fn join(&self) -> Result<(), Error> {
        if let Some(thread) = self.thread.lock().take() {
            if let Err(_) = thread.join() {
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
            .ok_or_else(|| format_err!("bad executable string"))?;

        Ok(format!("\"{}\" --silent", exe))
    }

    /// If the program is installed to run at startup.
    pub fn is_installed(&self) -> Result<bool, Error> {
        let key = self::registry::RegistryKey::current_user(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        )?;

        let path = match key.get("SetMod")? {
            Some(path) => path,
            None => return Ok(false),
        };

        Ok(Self::run_registry_entry()?.as_str() == path)
    }

    pub fn install(&self) -> Result<(), Error> {
        let key = self::registry::RegistryKey::current_user(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        )?;

        key.set("SetMod", &Self::run_registry_entry()?)?;
        Ok(())
    }

    pub fn uninstall(&self) -> Result<(), Error> {
        let key = self::registry::RegistryKey::current_user(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        )?;

        key.delete("SetMod")?;
        Ok(())
    }
}

/// Open the given directory.
fn open_dir(path: &Path) -> io::Result<bool> {
    use self::convert::ToWide as _;

    let path = path.to_wide_null();
    let operation = "open".to_wide_null();

    let result = unsafe {
        shellapi::ShellExecuteW(
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
    let restart_senders = Arc::new(Mutex::new(Vec::<oneshot::Sender<()>>::new()));
    let restart_senders1 = restart_senders.clone();

    // all senders to notify when we are shutting down.
    let shutdown_senders = Arc::new(Mutex::new(Vec::<oneshot::Sender<()>>::new()));
    let shutdown_senders1 = shutdown_senders.clone();

    let (events, mut events_rx) = mpsc::unbounded::<Event>();

    let window_loop = async move {
        let mut window = window::Window::new(String::from("SetMod")).await?;

        window.set_icon_from_buffer(ICON, 128, 128)?;

        window.add_menu_entry(0, "Open UI ...")?;
        window.add_menu_separator(1)?;
        window.add_menu_entry(2, "Open Log File ...")?;
        window.add_menu_entry(3, "Open Directory ...")?;
        window.add_menu_entry(4, "Restart Bot")?;
        window.add_menu_separator(5)?;
        window.add_menu_entry(6, "Quit Bot")?;

        loop {
            futures::select! {
                event = events_rx.select_next_some() => {
                    match event {
                        Event::Cleared => {
                            window.set_icon_from_buffer(ICON, 128, 128)?;
                        }
                        Event::Errored(message) => {
                            let message = format!("{}", message);
                            window.set_tooltip(&message)?;
                            window.set_icon_from_buffer(ICON_ERROR, 128, 128)?;
                        }
                        Event::Notification(n) => {
                            window.send_notification(n)?;
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
                                let _ = open_dir(&log_file)?;
                            }
                            3 => {
                                let _ = open_dir(&root)?;
                            }
                            4 => {
                                for tx in restart_senders1.lock().drain(..) {
                                    let _ = tx.send(());
                                }
                            }
                            6 => {
                                window.quit();

                                for tx in shutdown_senders1.lock().drain(..) {
                                    let _ = tx.send(());
                                }
                            }
                            _ => (),
                        },
                        window::Event::Shutdown => {
                            break;
                        }
                    }
                }
            }
        }

        for tx in shutdown_senders1.lock().drain(..) {
            let _ = tx.send(());
        }

        Ok::<_, io::Error>(())
    };

    let thread = thread::spawn(move || match futures::executor::block_on(window_loop) {
        Ok(()) => (),
        Err(e) => {
            log::error!("systray errored: {}", e);
        }
    });

    let system = System {
        thread: Arc::new(Mutex::new(Some(thread))),
        shutdown_senders,
        restart_senders,
        events,
        stopped: Arc::new(AtomicBool::new(false)),
    };

    Ok(system)
}
