use crate::web;
use failure::{bail, Error};
use futures::channel::oneshot;
use parking_lot::Mutex;
use std::{
    ffi::OsStr,
    io,
    os::windows::ffi::OsStrExt as _,
    path::Path,
    ptr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};
use winapi::um::{shellapi, winuser::SW_SHOW};

const ICON: &[u8] = include_bytes!("../../res/icon.ico");

#[derive(Clone)]
pub struct System {
    thread: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
    shutdown_senders: Arc<Mutex<Vec<oneshot::Sender<()>>>>,
    restart_senders: Arc<Mutex<Vec<oneshot::Sender<()>>>>,
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
}

/// Convert into a u16 string.
fn to_u16s(s: &OsStr) -> Vec<u16> {
    s.encode_wide()
        .chain(Some(0).into_iter())
        .collect::<Vec<_>>()
}

/// Open the given directory.
fn open_dir(path: &Path) -> io::Result<bool> {
    let path = to_u16s(path.as_os_str());
    let operation = to_u16s(OsStr::new("open"));

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
    let mut app = systray::Application::new()?;

    app.set_icon_from_buffer(ICON, 128, 128)?;

    let root = root.to_owned();
    let log_file = log_file.to_owned();

    app.add_menu_item("Open UI ...", move |_| {
        webbrowser::open(web::URL)?;
        Ok::<_, io::Error>(())
    })?;

    app.add_menu_separator()?;

    app.add_menu_item("Open Log File ...", move |_| {
        let _ = open_dir(&log_file)?;
        Ok::<_, io::Error>(())
    })?;

    app.add_menu_item("Open Directory ...", move |_| {
        let _ = open_dir(&root)?;
        Ok::<_, io::Error>(())
    })?;

    // all senders to notify when we are requesting a restart.
    let restart_senders = Arc::new(Mutex::new(Vec::<oneshot::Sender<()>>::new()));
    let restart_senders1 = restart_senders.clone();

    app.add_menu_separator()?;

    app.add_menu_item("Restart Bot", move |_| {
        for tx in restart_senders1.lock().drain(..) {
            let _ = tx.send(());
        }

        Ok::<_, systray::Error>(())
    })?;

    app.add_menu_separator()?;

    // all senders to notify when we are shutting down.
    let shutdown_senders = Arc::new(Mutex::new(Vec::<oneshot::Sender<()>>::new()));
    let shutdown_senders1 = shutdown_senders.clone();

    app.add_menu_item("Quit Bot", move |w| {
        for tx in shutdown_senders1.lock().drain(..) {
            let _ = tx.send(());
        }

        w.quit();
        Ok::<_, systray::Error>(())
    })?;

    let shutdown_senders2 = shutdown_senders.clone();

    let thread = thread::spawn(move || {
        if let Err(e) = app.wait_for_message() {
            log::warn!("systray handler crashed: {}", e);
        }

        for tx in shutdown_senders2.lock().drain(..) {
            let _ = tx.send(());
        }
    });

    Ok(System {
        thread: Arc::new(Mutex::new(Some(thread))),
        shutdown_senders,
        restart_senders,
        stopped: Arc::new(AtomicBool::new(false)),
    })
}
