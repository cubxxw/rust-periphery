use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::str::FromStr;
use std::io::{self, Read, Write};
use std::io::ErrorKind::Other;

#[derive(Debug)]
pub struct Pwm {
    chip: PwmChip,
    number: u32
}

#[derive(Debug)]
pub struct PwmChip {
    pub number: u32
}

#[derive(Debug)]
pub enum Polarity {
    Normal,
    Inverse
}

impl Pwm {
    /// Create a new Pwm wiht the provided chip/number
    ///
    /// This function does not export the Pwm pin
    pub fn new(chip: u32, number: u32) -> io::Result<Pwm> {
        let chip: PwmChip = PwmChip::new(chip)?;

        Ok(Pwm {
            chip,
            number,
        })
    }

    /// Export the Pwm for use
    pub fn export(&self) -> io::Result<()> {
        self.chip.export(self.number)
    }

    /// Unexport the PWM
    pub fn unexport(&self) -> io::Result<()> {
        self.chip.unexport(self.number)
    }

    /// Enable/Disable the PWM Signal
    pub fn enable(&self, enable: bool) -> io::Result<()> {
        let mut enable_file = pwm_file_wo(&self.chip, self.number, "enable")?;

        let contents = if enable { "1" } else { "0" };
        enable_file.write_all(contents.as_bytes())?;

        Ok(())
    }

    /// Query the state of enable for a given PWM pin
    pub fn enabled(&self) -> io::Result<bool> {
        pwm_file_parse::<u32>(&self.chip, self.number, "enable").map(|enable_state| {
            match enable_state {
                1 => true,
                0 => false,
                _ => panic!("enable != 1|0 should be unreachable"),
            }
        })
    }

    /// Get the currently configured duty_cycle in nanoseconds
    pub fn duty_cycle_ns(&self) -> io::Result<u32> {
        pwm_file_parse::<u32>(&self.chip, self.number, "duty_cycle")
    }

    /// The active time of the PWM signal
    ///
    /// Value is in nanoseconds and must be less than the period.
    pub fn set_duty_cycle_ns(&self, duty_cycle_ns: u32) -> io::Result<()> {
        // we'll just let the kernel do the validation
        let mut duty_cycle_file = pwm_file_wo(&self.chip, self.number, "duty_cycle")?;
        duty_cycle_file.write_all(format!("{}", duty_cycle_ns).as_bytes())?;
        Ok(())
    }

    /// Get the currently configured period in nanoseconds
    pub fn period_ns(&self) -> io::Result<u32> {
        pwm_file_parse::<u32>(&self.chip, self.number, "period")
    }

    /// The period of the PWM signal in Nanoseconds
    pub fn set_period_ns(&self, period_ns: u32) -> io::Result<()> {
        let mut period_file = pwm_file_wo(&self.chip, self.number, "period")?;
        period_file.write_all(format!("{}", period_ns).as_bytes())?;
        Ok(())
    }
}

impl PwmChip {
    pub fn new(number: u32) -> io::Result<PwmChip> {
        fs::metadata(&format!("/sys/class/pwm/pwmchip{}", number))?;
        Ok(PwmChip { number })
    }

    pub fn count(&self) -> io::Result<u32> {
        let npwm_path = format!("/sys/class/pwm/pwmchip{}/npwm", self.number);

        let mut s = String::new();
        File::open(&npwm_path)?.read_to_string(&mut s)?;

        match s.parse::<u32>() {
            Ok(n) => Ok(n),
            Err(_) => Err(io::Error::new(Other,
                format!("Unexpected npwm contents: {:?}", s)
            )),
        }
    }

    pub fn export(&self, number: u32) -> io::Result<()> {
        // only export if not already exported
        if fs::metadata(&format!(
            "/sys/class/pwm/pwmchip{}/pwm{}",
            self.number, number
        ))
        .is_err()
        {
            let path = format!("/sys/class/pwm/pwmchip{}/export", self.number);
            let _ = File::create(&path)?.write_all(format!("{}", number).as_bytes());
        }
        Ok(())
    }

    pub fn unexport(&self, number: u32) -> io::Result<()> {
        if fs::metadata(&format!(
            "/sys/class/pwm/pwmchip{}/pwm{}",
            self.number, number
        ))
        .is_ok()
        {
            let path = format!("/sys/class/pwm/pwmchip{}/unexport", self.number);
            let _ = File::create(&path)?.write_all(format!("{}", number).as_bytes());
        }
        Ok(())
    }
}

/// Open the specified entry name as a writable file
fn pwm_file_wo(chip: &PwmChip, pin: u32, name: &str) -> io::Result<File> {
    let f = OpenOptions::new().write(true).open(format!(
        "/sys/class/pwm/pwmchip{}/pwm{}/{}",
        chip.number, pin, name
    ))?;
    Ok(f)
}

/// Open the specified entry name as a readable file
fn pwm_file_ro(chip: &PwmChip, pin: u32, name: &str) -> io::Result<File> {
    let f = File::open(format!(
        "/sys/class/pwm/pwmchip{}/pwm{}/{}",
        chip.number, pin, name
    ))?;
    Ok(f)
}

/// Get the u32 value from the given entry
fn pwm_file_parse<T: FromStr>(chip: &PwmChip, pin: u32, name: &str) -> io::Result<T> {
    let mut s = String::with_capacity(10);
    let mut f = pwm_file_ro(chip, pin, name)?;
    f.read_to_string(&mut s)?;

    match s.trim().parse::<T>() {
        Ok(r) => Ok(r),
        Err(_) => Err(io::Error::new(Other,
            format!("Unexpeted value file contents: {:?}", s)
        )),
    }
}
