// Helper macro to execute a system call that returns an `io::Result`, providing more context in case of errors.
macro_rules! syscall {
    ($fn:ident($($arg:expr),* $(,)*)) => {{
        let res = unsafe { libc::$fn($($arg, )*) };
        if res == -1 {
            Err(std::io::Error::new(
                std::io::Error::last_os_error().kind(),
                format!("{} failed with {:?}", stringify!($fn), ($($arg, )*)),
            ))
        } else {
            Ok(res)
        }
    }};
}

pub mod gpio;
pub mod i2c;
pub mod spi;
pub mod pwm;
