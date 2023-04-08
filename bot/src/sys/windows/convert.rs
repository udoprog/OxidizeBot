use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt as _, OsStringExt};

pub(crate) trait ToWide {
    /// Encode into a wide string.
    fn to_wide(&self) -> Vec<u16>;

    /// Encode into a null-terminated wide string.
    fn to_wide_null(&self) -> Vec<u16>;
}

impl<T> ToWide for T
where
    T: AsRef<OsStr>,
{
    fn to_wide(&self) -> Vec<u16> {
        self.as_ref().encode_wide().collect()
    }

    fn to_wide_null(&self) -> Vec<u16> {
        self.as_ref().encode_wide().chain(Some(0)).collect()
    }
}

pub(crate) trait FromWide
where
    Self: Sized,
{
    fn from_wide(wide: &[u16]) -> Self;

    fn from_wide_null(wide: &[u16]) -> Self {
        let len = wide.iter().take_while(|&&c| c != 0).count();
        Self::from_wide(&wide[..len])
    }
}

impl FromWide for OsString {
    fn from_wide(wide: &[u16]) -> OsString {
        OsStringExt::from_wide(wide)
    }
}
