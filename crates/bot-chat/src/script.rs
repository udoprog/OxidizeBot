#[cfg(feature = "scripting")]
mod io;
mod mock;
#[cfg(feature = "scripting")]
mod real;

#[cfg(feature = "scripting")]
pub(crate) use self::real::*;

#[cfg(not(feature = "scripting"))]
pub(crate) use self::mock::*;
