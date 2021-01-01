#![allow(unused)]

use super::convert::{FromWide as _, ToWide as _};
use crate::prelude::*;
use crate::sys::Notification;
use anyhow::{anyhow, Context as _, Error};
use std::cell::RefCell;
use std::ffi::OsStr;
use std::future::Future;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::thread;
use std::time::Duration;
use winapi::shared::basetsd::ULONG_PTR;
use winapi::shared::minwindef::{
    DWORD, FALSE, HINSTANCE, LPARAM, LRESULT, PBYTE, TRUE, UINT, WPARAM,
};
use winapi::shared::ntdef::LPCWSTR;
use winapi::shared::windef::{HBRUSH, HICON, HMENU, HWND, POINT};
use winapi::um::libloaderapi;
use winapi::um::shellapi;
use winapi::um::winuser;
use winapi::um::winuser::{
    IMAGE_ICON, LR_DEFAULTCOLOR, LR_LOADFROMFILE, MENUINFO, MENUITEMINFOW, MFS_DEFAULT,
    MFT_SEPARATOR, MFT_STRING, MIIM_FTYPE, MIIM_ID, MIIM_STATE, MIIM_STRING, MIM_APPLYTOSUBMENUS,
    MIM_STYLE, MNS_NOTIFYBYPOS, WM_DESTROY, WM_USER, WNDCLASSW, WS_OVERLAPPEDWINDOW,
};

const ICON_MSG_ID: UINT = WM_USER + 1;

thread_local!(static WININFO_STASH: RefCell<Option<WindowsLoopData>> = RefCell::new(None));

/// Copy a wide string from a source to a destination.
pub fn copy_wstring(dest: &mut [u16], source: &str) {
    let source = source.to_wide_null();
    let len = usize::min(source.len(), dest.len());
    dest[..len].copy_from_slice(&source[..len]);
}

#[derive(Clone)]
struct WindowInfo {
    pub hwnd: HWND,
    pub hinstance: HINSTANCE,
    pub hmenu: HMENU,
}

unsafe impl Send for WindowInfo {}
unsafe impl Sync for WindowInfo {}

#[derive(Debug)]
pub enum Event {
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
    pub info: WindowInfo,
    pub events_tx: mpsc::UnboundedSender<Event>,
}

unsafe extern "system" fn window_proc(
    h_wnd: HWND,
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

                    winuser::SetForegroundWindow(h_wnd);

                    WININFO_STASH.with(|stash| {
                        let stash = stash.borrow();
                        let stash = stash.as_ref().expect("stash");

                        winuser::TrackPopupMenu(
                            stash.info.hmenu,
                            0,
                            p.x,
                            p.y,
                            (winuser::TPM_BOTTOMALIGN | winuser::TPM_LEFTALIGN) as i32,
                            h_wnd,
                            std::ptr::null_mut(),
                        );
                    });
                }
                _ => (),
            }
        }
        winuser::WM_DESTROY => {
            winuser::PostQuitMessage(0);
        }
        winuser::WM_MENUCOMMAND => {
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

    winuser::DefWindowProcW(h_wnd, msg, w_param, l_param)
}

fn new_nid(hwnd: HWND) -> shellapi::NOTIFYICONDATAW {
    let mut nid = shellapi::NOTIFYICONDATAW::default();
    nid.cbSize = std::mem::size_of::<shellapi::NOTIFYICONDATAW>() as DWORD;
    nid.hWnd = hwnd;
    nid.uID = 0x1 as UINT;
    nid
}

fn new_menuitem() -> MENUITEMINFOW {
    let mut info = MENUITEMINFOW::default();
    info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as UINT;
    info
}

unsafe fn init_window(name: &str) -> Result<WindowInfo, io::Error> {
    let class_name = name.to_wide_null();

    let hinstance: HINSTANCE = libloaderapi::GetModuleHandleA(std::ptr::null_mut());

    let wnd = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: 0 as HINSTANCE,
        hIcon: winuser::LoadIconW(0 as HINSTANCE, winuser::IDI_APPLICATION),
        hCursor: winuser::LoadCursorW(0 as HINSTANCE, winuser::IDI_APPLICATION),
        hbrBackground: 16 as HBRUSH,
        lpszMenuName: 0 as LPCWSTR,
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
        0 as HWND,
        0 as HMENU,
        0 as HINSTANCE,
        std::ptr::null_mut(),
    );

    if hwnd.is_null() {
        return Err(io::Error::last_os_error());
    }

    let mut nid = new_nid(hwnd);
    nid.uFlags = shellapi::NIF_MESSAGE;
    nid.uCallbackMessage = ICON_MSG_ID;

    let result = shellapi::Shell_NotifyIconW(
        shellapi::NIM_ADD,
        &mut nid as *mut shellapi::NOTIFYICONDATAW,
    );

    if result == FALSE {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Shell_NotifyIconW: failed",
        ));
    }

    // Setup menu
    let hmenu = winuser::CreatePopupMenu();

    let m = MENUINFO {
        cbSize: std::mem::size_of::<MENUINFO>() as DWORD,
        fMask: MIM_APPLYTOSUBMENUS | MIM_STYLE,
        dwStyle: MNS_NOTIFYBYPOS,
        cyMax: 0 as UINT,
        hbrBack: 0 as HBRUSH,
        dwContextHelpID: 0 as DWORD,
        dwMenuData: 0 as ULONG_PTR,
    };

    if winuser::SetMenuInfo(hmenu, &m as *const MENUINFO) == FALSE {
        return Err(io::Error::last_os_error());
    }

    Ok(WindowInfo {
        hwnd,
        hmenu,
        hinstance,
    })
}

unsafe fn run_loop() {
    let mut msg = winuser::MSG::default();

    loop {
        winuser::GetMessageW(&mut msg, 0 as HWND, 0, 0);

        if msg.message == winuser::WM_QUIT {
            break;
        }

        winuser::TranslateMessage(&msg);
        winuser::DispatchMessageW(&msg);
    }
}

/// A windows application window.
pub struct Window {
    info: WindowInfo,
    shutdown_rx: Fuse<oneshot::Receiver<()>>,
    events_rx: mpsc::UnboundedReceiver<Event>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Window {
    /// Construct a new window.
    pub async fn new(name: String) -> Result<Window, io::Error> {
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

            run_loop();

            if shutdown_tx.send(()).is_err() {
                log::error!("shutdown receiver closed");
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
    pub fn tick(&mut self) -> TickFuture<'_> {
        TickFuture { window: self }
    }

    pub fn quit(&mut self) {
        unsafe {
            winuser::PostMessageW(self.info.hwnd, WM_DESTROY, 0 as WPARAM, 0 as LPARAM);
        }

        if let Some(t) = self.thread.take() {
            t.join().expect("bye thread panicked");
        }
    }

    /// Set the tooltip we get when hovering over the systray icon.
    pub fn set_tooltip(&self, tooltip: &str) -> Result<(), Error> {
        self.raw_set_tooltip(tooltip)
            .with_context(|| anyhow!("failed to set tooltip `{}`", tooltip))
            .map_err(Error::from)
    }

    fn raw_set_tooltip(&self, tooltip: &str) -> Result<(), io::Error> {
        let mut nid = new_nid(self.info.hwnd);
        copy_wstring(&mut nid.szTip, tooltip);

        let result = unsafe {
            shellapi::Shell_NotifyIconW(
                shellapi::NIM_MODIFY,
                &mut nid as *mut shellapi::NOTIFYICONDATAW,
            )
        };

        if result == FALSE {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Shell_NotifyIconW: failed",
            ));
        }

        Ok(())
    }

    /// Add a menu entry.
    pub fn add_menu_entry(
        &self,
        item_idx: u32,
        item_name: &str,
        default: bool,
    ) -> Result<(), Error> {
        self.raw_add_menu_entry(item_idx, item_name, default)
            .with_context(|| anyhow!("failed to add meny entry {} `{}`", item_idx, item_name))
            .map_err(Error::from)
    }

    fn raw_add_menu_entry(
        &self,
        item_idx: u32,
        item_name: &str,
        default: bool,
    ) -> Result<(), io::Error> {
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

        let result = unsafe {
            winuser::InsertMenuItemW(self.info.hmenu, item_idx, 1, &item as *const MENUITEMINFOW)
        };

        if result == FALSE {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    /// Add a menu separator with the associated index.
    pub fn add_menu_separator(&self, item_idx: u32) -> Result<(), Error> {
        self.raw_add_menu_separator(item_idx)
            .with_context(|| anyhow!("failed to add menu separator"))
            .map_err(Error::from)
    }

    fn raw_add_menu_separator(&self, item_idx: u32) -> Result<(), io::Error> {
        let mut item = new_menuitem();
        item.fMask = MIIM_FTYPE;
        item.fType = MFT_SEPARATOR;
        item.wID = item_idx;

        let result = unsafe {
            winuser::InsertMenuItemW(self.info.hmenu, item_idx, 1, &item as *const MENUITEMINFOW)
        };

        if result == FALSE {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    /// Send a notification.
    pub fn send_notification(&self, n: Notification) -> Result<(), io::Error> {
        let mut nid = new_nid(self.info.hwnd);
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

        let result = unsafe {
            shellapi::Shell_NotifyIconW(
                shellapi::NIM_MODIFY,
                &mut nid as *mut shellapi::NOTIFYICONDATAW,
            )
        };

        if result == FALSE {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Shell_NotifyIconW: failed",
            ));
        }

        Ok(())
    }

    /// Set an icon from a resource.
    pub fn set_icon_from_resource(&self, resource_name: &str) -> Result<(), io::Error> {
        let resource_name = resource_name.to_wide_null();

        let icon = unsafe {
            winuser::LoadImageW(
                self.info.hinstance,
                resource_name.as_ptr(),
                IMAGE_ICON,
                64,
                64,
                0,
            )
        };

        if icon.is_null() {
            return Err(io::Error::last_os_error());
        }

        self.set_icon(icon as HICON)
    }

    /// Set the process icon from a file.
    pub fn set_icon_from_file(&self, icon_file: &str) -> Result<(), io::Error> {
        let wstr_icon_file = icon_file.to_wide_null();

        let hicon = unsafe {
            let result = winuser::LoadImageW(
                std::ptr::null_mut(),
                wstr_icon_file.as_ptr(),
                IMAGE_ICON,
                64,
                64,
                LR_LOADFROMFILE,
            );

            if result.is_null() {
                return Err(io::Error::last_os_error());
            }

            result as HICON
        };

        self.set_icon(hicon)
    }

    /// Set an icon from a buffer.
    pub fn set_icon_from_buffer(
        &self,
        buffer: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), Error> {
        self.raw_set_icon_from_buffer(buffer, width, height)
            .with_context(|| anyhow!("error setting icon from buffer"))
            .map_err(Error::from)
    }

    /// Set an icon from a buffer.
    fn raw_set_icon_from_buffer(
        &self,
        buffer: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), io::Error> {
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

    /// Shutdown the given window.
    fn shutdown(&self) -> Result<(), io::Error> {
        let result = unsafe {
            let mut nid = new_nid(self.info.hwnd);
            nid.uFlags = shellapi::NIF_ICON;

            shellapi::Shell_NotifyIconW(
                shellapi::NIM_DELETE,
                &mut nid as *mut shellapi::NOTIFYICONDATAW,
            )
        };

        if result == FALSE {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Shell_NotifyIconW: failed",
            ));
        }

        Ok(())
    }

    /// Internal call to set icon.
    fn set_icon(&self, icon: HICON) -> Result<(), io::Error> {
        let result = unsafe {
            let mut nid = new_nid(self.info.hwnd);
            nid.uFlags = shellapi::NIF_ICON;
            nid.hIcon = icon;

            shellapi::Shell_NotifyIconW(
                shellapi::NIM_MODIFY,
                &mut nid as *mut shellapi::NOTIFYICONDATAW,
            )
        };

        if result == FALSE {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Shell_NotifyIconW: failed",
            ));
        }

        Ok(())
    }
}

pub struct TickFuture<'a> {
    window: &'a mut Window,
}

impl<'a> Future for TickFuture<'a> {
    type Output = Event;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if let Poll::Ready(result) = Pin::new(&mut self.window.shutdown_rx).poll(ctx) {
            result.expect("shutdown receiver ended");
            return Poll::Ready(Event::Shutdown);
        }

        if let Poll::Ready(Some(event)) = self.window.events_rx.poll_recv(ctx) {
            return Poll::Ready(event);
        }

        Poll::Pending
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        self.shutdown().expect("shutdown failed");
    }
}
