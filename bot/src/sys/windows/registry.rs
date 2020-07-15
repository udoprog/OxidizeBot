use std::ffi::{OsStr, OsString};
use std::io;
use std::ptr;
use winapi::shared::{minwindef::HKEY, winerror};
use winapi::um::{winnt, winreg};

use super::convert::{FromWide as _, ToWide as _};

pub struct RegistryKey(HKEY);

unsafe impl Sync for RegistryKey {}
unsafe impl Send for RegistryKey {}

impl RegistryKey {
    /// Open the given key in the HKEY_CURRENT_USER scope.
    pub fn current_user(key: &str) -> io::Result<RegistryKey> {
        Self::open(winreg::HKEY_CURRENT_USER, key)
    }

    /// Internal open implementation.
    fn open(reg: HKEY, key: &str) -> io::Result<RegistryKey> {
        let key = key.to_wide_null();
        let mut ret = ptr::null_mut();

        let status = unsafe {
            winreg::RegOpenKeyExW(
                reg,
                key.as_ptr(),
                0,
                winnt::KEY_READ | winnt::KEY_SET_VALUE | winnt::KEY_WOW64_32KEY,
                &mut ret,
            )
        };

        if status != 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(RegistryKey(ret))
    }

    /// Get the given value.
    pub fn get(&self, name: &str) -> io::Result<Option<OsString>> {
        let name = name.to_wide_null();
        let mut len = 0;

        let status = unsafe {
            winreg::RegGetValueW(
                self.0,
                ptr::null_mut(),
                name.as_ptr(),
                winreg::RRF_RT_REG_SZ,
                ptr::null_mut(),
                ptr::null_mut(),
                &mut len,
            )
        };

        if status as u32 == winerror::ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }

        if status != 0 {
            return Err(io::Error::last_os_error());
        }

        let mut v = vec![0; len as usize / 2];

        let status = unsafe {
            winreg::RegGetValueW(
                self.0,
                ptr::null_mut(),
                name.as_ptr(),
                winreg::RRF_RT_REG_SZ,
                ptr::null_mut(),
                v.as_mut_ptr() as *mut _,
                &mut len,
            )
        };

        if status != 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(Some(OsString::from_wide_null(&v)))
    }

    /// Set the given value.
    pub fn set(&self, name: &str, value: impl AsRef<OsStr>) -> io::Result<()> {
        use std::convert::TryInto as _;

        let name = name.to_wide_null();
        let value = value.to_wide_null();
        let value_len: u32 = (value.len() * 2)
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "value too large"))?;

        let status = unsafe {
            winreg::RegSetValueExW(
                self.0,
                name.as_ptr(),
                0,
                winnt::REG_SZ,
                value.as_ptr() as *const u8,
                value_len,
            )
        };

        if status != 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    /// Delete the given value.
    pub fn delete(&self, name: &str) -> io::Result<()> {
        let name = name.to_wide_null();

        let status = unsafe { winreg::RegDeleteKeyValueW(self.0, ptr::null_mut(), name.as_ptr()) };

        if status != 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }
}

impl Drop for RegistryKey {
    fn drop(&mut self) {
        unsafe {
            winreg::RegCloseKey(self.0);
        }
    }
}
