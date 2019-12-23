use std::fs;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::io::{self, Write, Read};
use std::io::ErrorKind::{InvalidData, Other};

#[derive(Debug, Copy, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub enum Direction {
    In,
    Out,
    Low,
    High
}

#[derive(Debug, Copy, PartialEq, Eq, Clone, PartialOrd, Ord)]
#[repr(u8)]
pub enum Value {
    Low = 0,
    High = 1
}

#[derive(Debug, Copy, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub enum Edge {
    None, // 无中断触发
    Rising, // 上升沿触发
    Falling, // 下降沿触发
    Both // 上升、下降都沿
}

#[derive(Debug, Copy, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct Pin {
    pub num: usize
}

impl Pin {
    pub fn new(num: usize) -> Pin {
        Pin { num }
    }

    pub fn from_path<T: AsRef<Path>>(path: T) -> io::Result<Pin> {
        let pb = fs::canonicalize(path.as_ref())?;

        if !fs::metadata(&pb)?.is_dir() {
            return Err(io::Error::new(
                Other,
                "Provided path not a directory or symlink to a directory".to_owned()
            ));
        }
        let num = Pin::extract_pin_from_path(&pb)?;
        Ok(Pin::new(num))
    }

    fn extract_pin_from_path<P: AsRef<Path>>(path: P) -> io::Result<usize> {
        path.as_ref()
            .file_name()
            .and_then(|filename| filename.to_str())
            .and_then(|filename_str| filename_str.trim_start_matches("gpio").parse::<usize>().ok())
            .ok_or(io::Error::new(InvalidData, format!("{:?}", path.as_ref())))
    }

    fn write_sys_file(&self, file_name: &str, value: &str) -> io::Result<()> {
        let path = format!("/sys/class/gpio/gpio{}/{}", self.num, file_name);

        let mut file = OpenOptions::new().write(true).open(&path)?;
        file.write_all(value.as_bytes())?;
        
        Ok(())
    }

    fn read_sys_file(&self, file_name: &str) -> io::Result<String> {
        let path = format!("/sys/class/gpio/gpio{}/{}", self.num, file_name);

        let mut file = File::open(&path)?;
        let mut s = String::new();
        file.read_to_string(&mut s)?;

        Ok(s)
    }

    pub fn is_exported(&self) -> bool {
        fs::metadata(&format!("/sys/class/gpio/gpio{}", self.num)).is_ok()
    }

    pub fn export(&self) -> io::Result<()> {
        if !self.is_exported() {
            let mut file = OpenOptions::new().write(true).open("/sys/class/gpio/export")?;
            file.write_all(format!("{}", self.num).as_bytes())?;
        }

        Ok(())
    }

    pub fn unexport(&self) -> io::Result<()> {
        if self.is_exported() {
            let mut file = OpenOptions::new().write(true).open("/sys/class/gpio/unexport")?;
            file.write_all(format!("{}", self.num).as_bytes())?;
        }

        Ok(())
    }

    pub fn direction(&self) -> io::Result<Direction> {
        match self.read_sys_file("direction") {
            Ok(s) => {
                match s.trim() {
                    "in" => Ok(Direction::In),
                    "out" => Ok(Direction::Out),
                    "high" => Ok(Direction::High),
                    "low" => Ok(Direction::Low),
                    other => Err(io::Error::new(
                        Other,
                        format!("direction file contents {}", other)
                    ))
                }
            }
            Err(e) => Err(::std::convert::From::from(e)),
        }
    }

    pub fn set_direction(&self, dir: Direction) -> io::Result<()> {
        self.write_sys_file("direction", match dir {
                Direction::In => "in",
                Direction::Out => "out",
                Direction::High => "high",
                Direction::Low => "low"
            })?;

        Ok(())
    }

    pub fn value(&self) -> io::Result<Value> {
        match self.read_sys_file("value") {
            Ok(s) => {
                match s.trim() {
                    "1" => Ok(Value::High),
                    "0" => Ok(Value::Low),
                    other => Err(io::Error::new(
                        Other,
                        format!("value file contents {}", other)
                    ))
                }
            }
            Err(e) => Err(e),
        }
    }

    pub fn set_value(&self, value: Value) -> io::Result<()> {
        self.write_sys_file("value", match value {
                Value::Low => "0",
                Value::High => "1"
            })?;

        Ok(())
    }

    pub fn edge(&self) -> io::Result<Edge> {
        match self.read_sys_file("edge") {
            Ok(s) => {
                match s.trim() {
                    "none" => Ok(Edge::None),
                    "rising" => Ok(Edge::Rising),
                    "falling" => Ok(Edge::Falling),
                    "both" => Ok(Edge::Both),
                    other => Err(io::Error::new(
                        Other,
                        format!("Unexpected file contents {}", other)
                    ))
                }
            }
            Err(e) => Err(e),
        }
    }

    pub fn set_edge(&self, edge: Edge) -> io::Result<()> {
        self.write_sys_file("edge", match edge {
                Edge::None => "none",
                Edge::Rising => "rising",
                Edge::Falling => "falling",
                Edge::Both => "both"
            })?;

        Ok(())
    }

    pub fn active_low(&self) -> io::Result<bool> {
        match self.read_sys_file("active_low") {
            Ok(s) => {
                match s.trim() {
                    "1" => Ok(true),
                    "0" => Ok(false),
                    other => Err(io::Error::new(
                        Other,
                        format!("active_low file contents {}", other)
                    ))
                }
            }
            Err(e) => Err(e),
        }
    }

    pub fn set_active_low(&self, active_low: bool) -> io::Result<()> {
        self.write_sys_file("active_low",
                match active_low {
                    true => "1",
                    false => "0"
            })?;

        Ok(())
    }
}
