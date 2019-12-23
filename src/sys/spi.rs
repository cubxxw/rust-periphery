#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_macros)]

use std::io::{self, Read, Write};
use std::fs::{File, OpenOptions};
use std::marker::PhantomData;
use std::fmt;
use std::os::unix::io::{AsRawFd, RawFd};

// 125.0 MHz   125000000
// 62.5 MHz    62500000
// 31.2 MHz    31200000
// 15.6 MHz    15600000
// 7.8 MHz     7800000
// 3.9 MHz     3900000
// 1953 kHz    1953000
// 976 kHz     976000
// 488 kHz     488000
// 244 kHz     244000
// 122 kHz     122000
// 61 kHz  61000
// 30.5 kHz    30500
// 15.2 kHz    15200
// 7629 Hz     7629

pub struct SPI {
    file: File,
    _not_sync: PhantomData<*const ()>
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[repr(u8)]
pub enum Mode {
    Mode0 = 0,
    Mode1 = 1,
    Mode2 = 2,
    Mode3 = 3
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum BitOrder {
    MsbFirst = 0,
    LsbFirst = 1
}

/// Slave Select polarities.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Polarity {
    ActiveLow = 0,
    ActiveHigh = 1,
}

impl fmt::Display for Polarity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Polarity::ActiveLow => write!(f, "ActiveLow"),
            Polarity::ActiveHigh => write!(f, "ActiveHigh"),
        }
    }
}

impl fmt::Display for BitOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            BitOrder::MsbFirst => write!(f, "MsbFirst"),
            BitOrder::LsbFirst => write!(f, "LsbFirst")
        }
    }
}

pub type SpidevTransfer<'a, 'b> = private::spi_ioc_transfer<'a, 'b>;

impl SPI {
    pub fn new(bus: u8, slave: u8, speed_hz: u32, mode: Mode) -> io::Result<SPI> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(format!("/dev/spidev{}.{}", bus, slave))?;

        let spi = SPI {
            file,
             _not_sync: PhantomData
        };

        spi.set_mode(mode)?;
        spi.set_bits_per_word(8)?;
        spi.set_speed_hz(speed_hz)?;

        Ok(spi)
    }

    pub fn mode(&self) -> io::Result<Mode> {
        let mut mode: u8 = 0;

        private::get_mode_u8(self.file.as_raw_fd(), &mut mode)?;

        Ok(match mode & 0x03 {
            0x01 => Mode::Mode1,
            0x02 => Mode::Mode2,
            0x03 => Mode::Mode3,
            _ => Mode::Mode0,
        })
    }

    pub fn set_mode(&self, mode: Mode) -> io::Result<()> {
        let old_mode = self.mode()?;

        // Make sure we only replace the CPOL/CPHA bits
        let new_mode = ((old_mode as u8) & !0x03) | (mode as u8);

        private::set_mode_u8(self.file.as_raw_fd(), &new_mode)?;

        Ok(())
    }

    pub fn speed_hz(&self) -> io::Result<u32> {
        let mut speed_hz: u32 = 0;

        private::get_max_speed_hz(self.file.as_raw_fd(), &mut speed_hz)?;

        Ok(speed_hz)
    }

    pub fn set_speed_hz(&self, speed_hz: u32) -> io::Result<()> {
        private::set_max_speed_hz(self.file.as_raw_fd(), &speed_hz)?;

        Ok(())
    }

    pub fn bits_per_word(&self) -> io::Result<u8> {
        let mut bits_per_word: u8 = 0;

        private::get_bits_per_word(self.file.as_raw_fd(), &mut bits_per_word)?;

        Ok(bits_per_word)
    }

    pub fn set_bits_per_word(&self, size: u8) -> io::Result<()> {
        private::set_bits_per_word(self.file.as_raw_fd(), &size)?;

        Ok(())
    }

    pub fn bit_order(&self) -> io::Result<BitOrder> {
        let mut bit_order: u8 = 0;

        private::get_lsb_first(self.file.as_raw_fd(), &mut bit_order)?;

        Ok(match bit_order {
            0 => BitOrder::MsbFirst,
            _ => BitOrder::LsbFirst,
        })
    }

    pub fn set_bit_order(&self, bit_order: BitOrder) -> io::Result<()> {
        private::set_lsb_first(self.file.as_raw_fd(), &(bit_order as u8))?;

        Ok(())
    }

    pub fn ss_polarity(&self) -> io::Result<Polarity> {
        let mut mode: u8 = 0;

        private::get_mode_u8(self.file.as_raw_fd(), &mut mode)?;

        if (mode & private::SPI_CS_HIGH) == 0 {
            return Ok(Polarity::ActiveLow)
        }

        Ok(Polarity::ActiveHigh)
    }

    pub fn set_ss_polarity(&self, polarity: Polarity) -> io::Result<()> {
        let mut mode: u8 = 0;

        private::get_mode_u8(self.file.as_raw_fd(), &mut mode)?;

        if polarity == Polarity::ActiveHigh {
            mode |= private::SPI_CS_HIGH;
        } else {
            mode &= !private::SPI_CS_HIGH;
        }

        private::set_mode_u8(self.file.as_raw_fd(), &mode)?;

        Ok(())
    }

    pub fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        Ok(self.file.read(buffer)?)
    }

    pub fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        Ok(self.file.write(buffer)?)
    }

    pub fn transfer(&self, transfer: &mut SpidevTransfer) -> io::Result<()> {
        // The kernel will directly modify the rx_buf of the SpidevTransfer
        // rx_buf if present, so there is no need to do any additional work
        private::spidev_transfer(self.file.as_raw_fd(), transfer)?;

        Ok(())
    }

    pub fn transfer_multiple(&self, transfers: &mut [SpidevTransfer]) -> io::Result<()> {
        private::spidev_transfer_buf(self.file.as_raw_fd(), transfers)?;

        Ok(())
    }
}

impl AsRawFd for SPI {
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

impl fmt::Debug for SPI {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SPI")
            .field("mode", &self.mode())
            .field("speed_hz", &self.speed_hz())
            .field("bits_per_word", &self.bits_per_word())
            .field("bit_order", &self.bit_order())
            .field("ss_polarity", &self.ss_polarity())
            .finish()
    }
}

mod private {
    use std::os::raw::{c_int, c_ulong};

    #[cfg(target_env = "gnu")]
    type IoctlNumType = c_ulong;
    #[cfg(target_env = "musl")]
    type IoctlNumType = c_int;

    /// Clock Phase
    pub const SPI_CPHA: u8 = 0x01;
    /// Clock Polarity
    pub const SPI_CPOL: u8 = 0x02;

    pub const SPI_MODE_0: u8 = 0;
    pub const SPI_MODE_1: u8 = SPI_CPHA;
    pub const SPI_MODE_2: u8 = SPI_CPOL;
    pub const SPI_MODE_3: u8 = SPI_CPOL | SPI_CPHA;

    /// Chipselect Active High?
    pub const SPI_CS_HIGH: u8 = 0x04;
    /// Per-word Bits On Wire
    pub const SPI_LSB_FIRST: u8 = 0x08;
    /// SI/SO Signals Shared
    pub const SPI_3WIRE: u8 = 0x10;
    /// Loopback Mode
    pub const SPI_LOOP: u8 = 0x20;
    /// 1 dev/bus; no chipselect
    pub const SPI_NO_CS: u8 = 0x40;
    /// Slave pulls low to pause
    pub const SPI_READY: u8 = 0x80;

    /// Transmit with 2 wires
    pub const SPI_TX_DUAL: u32 = 0x100;
    /// Transmit with 4 wires
    pub const SPI_TX_QUAD: u32 = 0x200;
    /// Receive with 2 wires
    pub const SPI_RX_DUAL: u32 = 0x400;
    /// Receive with 4 wires
    pub const SPI_RX_QUAD: u32 = 0x800;

    const SPI_IOC_MAGIC: u8 = 'k' as u8;
    const SPI_IOC_NR_TRANSFER: u8 = 0;
    const SPI_IOC_NR_MODE: u8 = 1;
    const SPI_IOC_NR_LSB_FIRST: u8 = 2;
    const SPI_IOC_NR_BITS_PER_WORD: u8 = 3;
    const SPI_IOC_NR_MAX_SPEED_HZ: u8 = 4;
    const SPI_IOC_NR_MODE32: u8 = 5;

    const NONE: u8 = 0;
    const READ: u8 = 2;
    const WRITE: u8 = 1;
    const SIZEBITS: u8 = 14;
    const DIRBITS: u8 = 2;

    const NRBITS: IoctlNumType = 8;
    const TYPEBITS: IoctlNumType = 8;

    const NRSHIFT: IoctlNumType = 0;
    const TYPESHIFT: IoctlNumType = NRSHIFT + NRBITS as IoctlNumType;
    const SIZESHIFT: IoctlNumType = TYPESHIFT + TYPEBITS as IoctlNumType;
    const DIRSHIFT: IoctlNumType = SIZESHIFT + SIZEBITS as IoctlNumType;

    const NRMASK: IoctlNumType = (1 << NRBITS) - 1;
    const TYPEMASK: IoctlNumType = (1 << TYPEBITS) - 1;
    const SIZEMASK: IoctlNumType = (1 << SIZEBITS) - 1;
    const DIRMASK: IoctlNumType = (1 << DIRBITS) - 1;

    macro_rules! ioc {
        ($dir:expr, $ty:expr, $nr:expr, $sz:expr) => (
            (($dir as IoctlNumType & DIRMASK) << DIRSHIFT) |
            (($ty as IoctlNumType & TYPEMASK) << TYPESHIFT) |
            (($nr as IoctlNumType & NRMASK) << NRSHIFT) |
            (($sz as IoctlNumType & SIZEMASK) << SIZESHIFT))
    }

    // const SPI_IOC_RD_MODE: IoctlNumType = ioc!(READ, SPI_IOC_MAGIC, SPI_IOC_NR_MODE, mem::size_of::<u8>());

    macro_rules! request_code_none {
        ($ty:expr, $nr:expr) => (ioc!(NONE, $ty, $nr, 0))
    }

    macro_rules! request_code_read {
        ($ty:expr, $nr:expr, $sz:expr) => (ioc!(READ, $ty, $nr, $sz))
    }

    macro_rules! request_code_write {
        ($ty:expr, $nr:expr, $sz:expr) => (ioc!(WRITE, $ty, $nr, $sz))
    }

    macro_rules! request_code_readwrite {
        ($ty:expr, $nr:expr, $sz:expr) => (ioc!(READ | WRITE, $ty, $nr, $sz))
    }

    macro_rules! ioctl_read {
        ($(#[$attr:meta])* $name:ident, $ioty:expr, $nr:expr, $ty:ty) => (
            $(#[$attr])*
            pub fn $name(fd: std::os::raw::c_int, data: *mut $ty) -> std::io::Result<std::os::raw::c_int> {
                syscall!(ioctl(fd, request_code_read!($ioty, $nr, std::mem::size_of::<$ty>()) as IoctlNumType, data))
            }
        )
    }

    macro_rules! ioctl_write_ptr {
        ($(#[$attr:meta])* $name:ident, $ioty:expr, $nr:expr, $ty:ty) => (
            $(#[$attr])*
            pub fn $name(fd: std::os::raw::c_int, data: *const $ty) -> std::io::Result<std::os::raw::c_int> {
                syscall!(ioctl(fd, request_code_write!($ioty, $nr, std::mem::size_of::<$ty>()) as IoctlNumType, data))
            }
        )
    }

    macro_rules! ioctl_read_buf {
        ($(#[$attr:meta])* $name:ident, $ioty:expr, $nr:expr, $ty:ty) => (
            $(#[$attr])*
            pub fn $name(fd: std::os::raw::c_int,
                                data: &mut [$ty])
                                -> std::io::Result<std::os::raw::c_int> {
                syscall!(ioctl(fd, request_code_read!($ioty, $nr, data.len() * ::std::mem::size_of::<$ty>()) as IoctlNumType, data))
            }
        )
    }

    macro_rules! ioctl_write_buf {
        ($(#[$attr:meta])* $name:ident, $ioty:expr, $nr:expr, $ty:ty) => (
            $(#[$attr])*
            pub fn $name(fd: std::os::raw::c_int, data: &[$ty]) -> std::io::Result<std::os::raw::c_int> {
                syscall!(ioctl(fd, request_code_write!($ioty, $nr, data.len() * ::std::mem::size_of::<$ty>()) as IoctlNumType, data))
            }
        )
    }

    macro_rules! ioctl_readwrite_buf {
        ($(#[$attr:meta])* $name:ident, $ioty:expr, $nr:expr, $ty:ty) => (
            $(#[$attr])*
            pub fn $name(fd: std::os::raw::c_int,
                                data: &mut [$ty])
                                -> std::io::Result<std::os::raw::c_int> {
                syscall!(ioctl(fd, request_code_readwrite!($ioty, $nr, data.len() * ::std::mem::size_of::<$ty>()) as IoctlNumType, data))
            }
        )
    }

    ioctl_read!(get_mode_u8, SPI_IOC_MAGIC, SPI_IOC_NR_MODE, u8);
    ioctl_write_ptr!(set_mode_u8, SPI_IOC_MAGIC, SPI_IOC_NR_MODE, u8);
    ioctl_read!(get_mode_u32, SPI_IOC_MAGIC, SPI_IOC_NR_MODE32, u32);
    ioctl_write_ptr!(set_mode32, SPI_IOC_MAGIC, SPI_IOC_NR_MODE32, u32);

    ioctl_read!(get_lsb_first, SPI_IOC_MAGIC, SPI_IOC_NR_LSB_FIRST, u8);
    ioctl_write_ptr!(set_lsb_first, SPI_IOC_MAGIC, SPI_IOC_NR_LSB_FIRST, u8);

    ioctl_read!(get_bits_per_word, SPI_IOC_MAGIC, SPI_IOC_NR_BITS_PER_WORD, u8);
    ioctl_write_ptr!(set_bits_per_word, SPI_IOC_MAGIC, SPI_IOC_NR_BITS_PER_WORD, u8);

    ioctl_read!(get_max_speed_hz, SPI_IOC_MAGIC, SPI_IOC_NR_MAX_SPEED_HZ, u32);
    ioctl_write_ptr!(set_max_speed_hz, SPI_IOC_MAGIC, SPI_IOC_NR_MAX_SPEED_HZ, u32);

    #[allow(non_camel_case_types)]
    #[derive(Debug, Default)]
    #[repr(C)]
    pub struct spi_ioc_transfer<'a, 'b> {
        tx_buf: u64,
        rx_buf: u64,
        len: u32,

        // optional overrides
        pub speed_hz: u32,
        pub delay_usecs: u16,
        pub bits_per_word: u8,
        pub cs_change: u8,
        pub pad: u32,

        tx_buf_ref: std::marker::PhantomData<&'a [u8]>,
        rx_buf_ref: std::marker::PhantomData<&'b mut [u8]>,
    }

    impl<'a, 'b> spi_ioc_transfer<'a, 'b> {
        pub fn read(buff: &'b mut [u8]) -> Self {
            spi_ioc_transfer {
                rx_buf: buff.as_ptr() as *const () as usize as u64,
                len: buff.len() as u32,
                ..Default::default()
            }
        }

        pub fn write(buff: &'a [u8]) -> Self {
            spi_ioc_transfer {
                tx_buf: buff.as_ptr() as *const () as usize as u64,
                len: buff.len() as u32,
                ..Default::default()
            }
        }

        /// The `tx_buf` and `rx_buf` must be the same length.
        pub fn read_write(tx_buf: &'a [u8], rx_buf: &'b mut [u8]) -> Self {
            assert_eq!(tx_buf.len(), rx_buf.len());
            spi_ioc_transfer {
                rx_buf: rx_buf.as_ptr() as *const () as usize as u64,
                tx_buf: tx_buf.as_ptr() as *const () as usize as u64,
                len: tx_buf.len() as u32,
                ..Default::default()
            }
        }
    }


    ioctl_write_ptr!(spidev_transfer, SPI_IOC_MAGIC, SPI_IOC_NR_TRANSFER, spi_ioc_transfer);
    ioctl_write_buf!(spidev_transfer_buf, SPI_IOC_MAGIC, SPI_IOC_NR_TRANSFER, spi_ioc_transfer);
}
