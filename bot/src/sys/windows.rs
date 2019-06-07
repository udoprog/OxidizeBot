use crate::web;
use failure::{bail, Error};
use futures::channel::oneshot;
use parking_lot::Mutex;
use std::{ffi::OsStr, io, os::windows::ffi::OsStrExt as _, path::Path, ptr, sync::Arc, thread};
use winapi::um::{shellapi, winuser::SW_SHOW};

const ICON: &[u8] = include_bytes!("../../res/icon.ico");

#[derive(Clone)]
pub struct System {
    thread: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
    shutdown_rx: Arc<Mutex<Option<oneshot::Receiver<()>>>>,
}

impl System {
    /// Wait for system shutdown signal.
    pub async fn wait_for_shutdown(&self) -> Result<(), oneshot::Canceled> {
        let rx = self.shutdown_rx.lock().take();

        if let Some(rx) = rx {
            rx.await?;
        }

        Ok(())
    }

    /// Test if system is running.
    pub fn is_running(&self) -> bool {
        self.shutdown_rx.lock().is_some()
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

pub fn setup(root: &Path) -> Result<System, Error> {
    let mut app = systray::Application::new()?;
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    app.set_icon_from_buffer(ICON, 128, 128)?;

    let root = root.to_owned();

    app.add_menu_item("Open UI ...", move |_| {
        webbrowser::open(web::URL)?;
        Ok::<_, io::Error>(())
    })?;

    app.add_menu_item("Open Config Directory ...", move |_| {
        let _ = open_dir(&root)?;
        Ok::<_, io::Error>(())
    })?;

    app.add_menu_separator()?;

    let mut shutdown_tx = Some(shutdown_tx);

    app.add_menu_item("Quit", move |w| {
        if let Some(tx) = shutdown_tx.take() {
            w.quit();
            let _ = tx.send(());
        }

        Ok::<_, systray::Error>(())
    })?;

    let thread = thread::spawn(move || {
        if let Err(e) = app.wait_for_message() {
            panic!("systray handler crashed: {}", e);
        }
    });

    Ok(System {
        thread: Arc::new(Mutex::new(Some(thread))),
        shutdown_rx: Arc::new(Mutex::new(Some(shutdown_rx))),
    })
}
