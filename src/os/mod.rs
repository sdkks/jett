pub(crate) mod desktop;

#[cfg(target_os = "windows")]
pub(crate) mod windows;

#[cfg(not(target_os = "windows"))]
pub(crate) mod unix;
