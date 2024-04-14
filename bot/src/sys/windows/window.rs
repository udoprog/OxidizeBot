use std::cell::RefCell;
use std::io;
use std::ptr;
use std::thread;

use async_fuse::Fuse;
use tokio::sync::{mpsc, oneshot};
use winapi::shared::minwindef::{DWORD, FALSE, LPARAM, LRESULT, PBYTE, TRUE, UINT, WPARAM};
use winapi::shared::windef::{HBRUSH, HICON, HMENU, HWND, POINT};
use winapi::um::shellapi;
use winapi::um::winuser;
use winapi::um::winuser::{
    LR_DEFAULTCOLOR, MENUINFO, MENUITEMINFOW, MFS_DEFAULT, MFT_SEPARATOR, MFT_STRING, MIIM_FTYPE,
    MIIM_ID, MIIM_STATE, MIIM_STRING, MIM_APPLYTOSUBMENUS, MIM_STYLE, MNS_NOTIFYBYPOS, WM_DESTROY,
    WM_USER, WNDCLASSW, WS_OVERLAPPEDWINDOW,
};

use super::convert::ToWide as _;
use crate::sys::Notification;

const ICON_MSG_ID: UINT = WM_USER + 1;

thread_local!(static WININFO_STASH: RefCell<Option<WindowsLoopData>> = const { RefCell::new(None) });

/// Copy a wide string from a source to a destination.
pub(crate) fn copy_wstring(dest: &mut [u16], source: &str) {
    let source = source.to_wide_null();
    let len = usize::min(source.len(), dest.len());
    dest[..len].copy_from_slice(&source[..len]);
}

#[derive(Clone)]
struct WindowInfo {
    pub(crate) hwnd: HWND,
    pub(crate) hmenu: HMENU,
}

impl WindowInfo {
    fn new_nid(&self) -> shellapi::NOTIFYICONDATAW {
        let mut nid = shellapi::NOTIFYICONDATAW::default();
        nid.cbSize = std::mem::size_of::<shellapi::NOTIFYICONDATAW>() as DWORD;
        nid.hWnd = self.hwnd;
        nid.uID = 0x1 as UINT;
        nid
    }

    fn add_icon(&self) -> io::Result<()> {
        let mut nid = self.new_nid();
        nid.uFlags = shellapi::NIF_MESSAGE;
        nid.uCallbackMessage = ICON_MSG_ID;

        let result = unsafe { shellapi::Shell_NotifyIconW(shellapi::NIM_ADD, &mut nid) };

        if result == FALSE {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    fn delete_icon(&self) -> io::Result<()> {
        let result = unsafe {
            let mut nid = self.new_nid();
            nid.uFlags = shellapi::NIF_ICON;

            shellapi::Shell_NotifyIconW(shellapi::NIM_DELETE, &mut nid)
        };

        if result == FALSE {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }
}

unsafe impl Send for WindowInfo {}
unsafe impl Sync for WindowInfo {}

#[derive(Debug)]
pub(crate) enum Event {
    /// A meny item was clicked.
    MenuClicked(u32),
    /// Shutdown was requested.
    Shutdown,
    /// Balloon was clicked.
    BalloonClicked,
    /// Balloon timed out.
    BalloonTimeout,
}

#[derive(Clone)]
struct WindowsLoopData {
    pub(crate) info: WindowInfo,
    pub(crate) events_tx: mpsc::UnboundedSender<Event>,
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match msg {
        ICON_MSG_ID => {
            match l_param as UINT {
                // clicked balloon
                shellapi::NIN_BALLOONUSERCLICK => {
                    WININFO_STASH.with(|stash| {
                        let stash = stash.borrow();
                        let stash = stash.as_ref().expect("stash");

                        stash
                            .events_tx
                            .send(Event::BalloonClicked)
                            .expect("events sender to be open");
                    });
                }
                shellapi::NIN_BALLOONHIDE => {}
                shellapi::NIN_BALLOONTIMEOUT => {
                    WININFO_STASH.with(|stash| {
                        let stash = stash.borrow();
                        let stash = stash.as_ref().expect("stash");

                        stash
                            .events_tx
                            .send(Event::BalloonTimeout)
                            .expect("events sender to be open");
                    });
                }
                winuser::WM_LBUTTONUP | winuser::WM_RBUTTONUP => {
                    let mut p = POINT::default();

                    if winuser::GetCursorPos(&mut p as *mut POINT) == FALSE {
                        return 1;
                    }

                    winuser::SetForegroundWindow(hwnd);

                    WININFO_STASH.with(|stash| {
                        let stash = stash.borrow();
                        let stash = stash.as_ref().expect("stash");

                        winuser::TrackPopupMenu(
                            stash.info.hmenu,
                            0,
                            p.x,
                            p.y,
                            (winuser::TPM_BOTTOMALIGN | winuser::TPM_LEFTALIGN) as i32,
                            hwnd,
                            ptr::null_mut(),
                        );
                    });
                }
                _ => (),
            }
        }
        winuser::WM_DESTROY => {
            tracing::trace!("Got destroy message");
            winuser::PostQuitMessage(0);
        }
        winuser::WM_MENUCOMMAND => {
            tracing::trace!("Got menu command");
            WININFO_STASH.with(|stash| {
                let stash = stash.borrow();
                let stash = stash.as_ref().expect("stash");
                let menu_id = winuser::GetMenuItemID(stash.info.hmenu, w_param as i32) as i32;
                if menu_id != -1 {
                    stash
                        .events_tx
                        .send(Event::MenuClicked(menu_id as u32))
                        .expect("events sender to be open");
                }
            });
        }
        _ => (),
    }

    winuser::DefWindowProcW(hwnd, msg, w_param, l_param)
}

fn new_menuitem() -> MENUITEMINFOW {
    let mut info = MENUITEMINFOW::default();
    info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as UINT;
    info
}

unsafe fn init_window(name: &str) -> io::Result<WindowInfo> {
    let class_name = name.to_wide_null();

    let wnd = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: ptr::null_mut(),
        hIcon: winuser::LoadIconW(ptr::null_mut(), winuser::IDI_APPLICATION),
        hCursor: winuser::LoadCursorW(ptr::null_mut(), winuser::IDI_APPLICATION),
        hbrBackground: 16 as HBRUSH,
        lpszMenuName: ptr::null(),
        lpszClassName: class_name.as_ptr(),
    };

    if winuser::RegisterClassW(&wnd) == 0 {
        return Err(io::Error::last_os_error());
    }

    let name = name.to_wide_null();

    let hwnd = winuser::CreateWindowExW(
        0,
        class_name.as_ptr(),
        name.as_ptr(),
        WS_OVERLAPPEDWINDOW,
        winuser::CW_USEDEFAULT,
        0,
        winuser::CW_USEDEFAULT,
        0,
        ptr::null_mut(),
        ptr::null_mut(),
        ptr::null_mut(),
        ptr::null_mut(),
    );

    if hwnd.is_null() {
        return Err(io::Error::last_os_error());
    }

    // Setup menu
    let hmenu = winuser::CreatePopupMenu();

    let m = MENUINFO {
        cbSize: std::mem::size_of::<MENUINFO>() as DWORD,
        fMask: MIM_APPLYTOSUBMENUS | MIM_STYLE,
        dwStyle: MNS_NOTIFYBYPOS,
        cyMax: 0,
        hbrBack: ptr::null_mut(),
        dwContextHelpID: 0,
        dwMenuData: 0,
    };

    if winuser::SetMenuInfo(hmenu, &m) == FALSE {
        return Err(io::Error::last_os_error());
    }

    let info = WindowInfo { hwnd, hmenu };
    info.add_icon()?;
    Ok(info)
}

/// A windows application window.
pub(crate) struct Window {
    info: WindowInfo,
    shutdown_rx: Fuse<oneshot::Receiver<()>>,
    events_rx: mpsc::UnboundedReceiver<Event>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Window {
    /// Construct a new window.
    pub(crate) async fn new(name: String) -> io::Result<Window> {
        let (tx, rx) = oneshot::channel();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let (events_tx, events_rx) = mpsc::unbounded_channel();

        let thread = thread::spawn(move || unsafe {
            let info = match init_window(name.as_str()) {
                Ok(info) => info,
                Err(e) => {
                    if tx.send(Err(e)).is_err() {
                        panic!("failed to send error information to parent thread");
                    }

                    return;
                }
            };

            if tx.send(Ok(info.clone())).is_err() {
                panic!("failed to send window information to parent thread");
            }

            WININFO_STASH.with(|stash| {
                let data = WindowsLoopData {
                    info: info.clone(),
                    events_tx,
                };
                (*stash.borrow_mut()) = Some(data);
            });

            let mut msg = winuser::MSG::default();

            loop {
                winuser::GetMessageW(&mut msg, ptr::null_mut(), 0, 0);

                if msg.message == winuser::WM_QUIT {
                    break;
                }

                winuser::TranslateMessage(&msg);
                winuser::DispatchMessageW(&msg);
            }

            if let Err(error) = info.delete_icon() {
                tracing::error!("Failed to remove icon: {error}");
            }

            if shutdown_tx.send(()).is_err() {
                tracing::error!("Shutdown receiver closed");
            }
        });

        let info = rx
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "canceled"))??;

        let w = Window {
            info,
            shutdown_rx: Fuse::new(shutdown_rx),
            events_rx,
            thread: Some(thread),
        };

        Ok(w)
    }

    /// Tick the window a single event cycle.
    pub(crate) async fn tick(&mut self) -> Event {
        tokio::select! {
            _ = &mut self.shutdown_rx => {
                Event::Shutdown
            }
            event = self.events_rx.recv() => {
                event.unwrap_or(Event::Shutdown)
            }
        }
    }

    /// Join the current window.
    pub(crate) fn join(&mut self) {
        let result = unsafe { winuser::PostMessageW(self.info.hwnd, WM_DESTROY, 0, 0) };

        if result == FALSE {
            tracing::warn!(
                "Failed to post destroy message: {}",
                io::Error::last_os_error()
            );
        }

        if let Some(t) = self.thread.take() {
            if t.join().is_err() {
                tracing::warn!("Window thread panicked");
            }
        }
    }

    /// Set tooltip.
    pub(crate) fn set_tooltip(&self, tooltip: &str) -> io::Result<()> {
        let mut nid = self.info.new_nid();
        copy_wstring(&mut nid.szTip, tooltip);

        let result = unsafe { shellapi::Shell_NotifyIconW(shellapi::NIM_MODIFY, &mut nid) };

        if result == FALSE {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    /// Add a menu entry.
    pub(crate) fn add_menu_entry(
        &self,
        item_idx: u32,
        item_name: &str,
        default: bool,
    ) -> io::Result<()> {
        let mut st = item_name.to_wide_null();
        let mut item = new_menuitem();
        item.fMask = MIIM_FTYPE | MIIM_STRING | MIIM_ID | MIIM_STATE;
        item.fType = MFT_STRING;
        item.wID = item_idx;
        item.dwTypeData = st.as_mut_ptr();
        item.cch = (item_name.len() * 2) as u32;

        if default {
            item.fState = MFS_DEFAULT;
        }

        let result = unsafe { winuser::InsertMenuItemW(self.info.hmenu, item_idx, 1, &item) };

        if result == FALSE {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    /// Add a menu separator at the given index.
    pub(crate) fn add_menu_separator(&self, item_idx: u32) -> io::Result<()> {
        let mut item = new_menuitem();
        item.fMask = MIIM_FTYPE;
        item.fType = MFT_SEPARATOR;
        item.wID = item_idx;

        let result = unsafe { winuser::InsertMenuItemW(self.info.hmenu, item_idx, 1, &item) };

        if result == FALSE {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    /// Send a notification.
    pub(crate) fn send_notification(&self, n: Notification) -> io::Result<()> {
        let mut nid = self.info.new_nid();
        nid.uFlags = shellapi::NIF_INFO;

        if let Some(title) = n.title {
            copy_wstring(&mut nid.szInfoTitle, title.as_str());
        }

        copy_wstring(&mut nid.szInfo, n.message.as_str());

        if let Some(timeout) = n.timeout {
            unsafe {
                *nid.u.uTimeout_mut() = timeout.as_millis() as u32;
            }
        }

        nid.dwInfoFlags = n.icon.into_flags();

        let result = unsafe { shellapi::Shell_NotifyIconW(shellapi::NIM_MODIFY, &mut nid) };

        if result == FALSE {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    /// Set an icon from a buffer.
    pub(crate) fn set_icon_from_buffer(
        &self,
        buffer: &[u8],
        width: u32,
        height: u32,
    ) -> io::Result<()> {
        let offset = unsafe {
            winuser::LookupIconIdFromDirectoryEx(
                buffer.as_ptr() as PBYTE,
                TRUE,
                width as i32,
                height as i32,
                LR_DEFAULTCOLOR,
            )
        };

        if offset == 0 {
            return Err(io::Error::last_os_error());
        }

        let icon_data = &buffer[offset as usize..];

        let hicon = unsafe {
            winuser::CreateIconFromResourceEx(
                icon_data.as_ptr() as PBYTE,
                icon_data.len() as DWORD,
                TRUE,
                0x30000,
                width as i32,
                height as i32,
                LR_DEFAULTCOLOR,
            )
        };

        if hicon.is_null() {
            return Err(io::Error::last_os_error());
        }

        self.set_icon(hicon)
    }

    /// Internal call to set icon.
    fn set_icon(&self, icon: HICON) -> io::Result<()> {
        let result = unsafe {
            let mut nid = self.info.new_nid();
            nid.uFlags = shellapi::NIF_ICON;
            nid.hIcon = icon;

            shellapi::Shell_NotifyIconW(shellapi::NIM_MODIFY, &mut nid)
        };

        if result == FALSE {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }
}
