use std::{
    ffi::{OsStr, OsString}, process::{Command, Stdio},
};

use queue::Queue;

pub struct SystemLine {
    pub cpu_data: Queue<f64>,
    pub memory_data: Queue<f64>,
    pub network_data: Queue<f64>,
    pub swap_data: Queue<f64>,
}

#[derive(Clone)]
pub struct Config {
    pub services: Vec<String>,
}

impl Config {
    pub fn new() -> Config {
        Config {
            services: Vec::new(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemLine {
    pub fn new() -> SystemLine {
        SystemLine {
            cpu_data: queue::Queue::with_capacity(20),
            memory_data: queue::Queue::with_capacity(20),
            network_data: queue::Queue::with_capacity(20),
            swap_data: queue::Queue::with_capacity(20),
        }
    }
}

impl Default for SystemLine {
    fn default() -> Self {
        Self::new()
    }
}

pub struct HumanBytes<T: Copy + Into<u64>>(pub T);

impl<T: Copy + Into<u64>> std::fmt::Display for HumanBytes<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const UNITS: [&str; 7] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];

        let bytes = self.0.into() as f64;
        let i = ((bytes.log2() / 10.0) as usize).min(UNITS.len() - 1);
        let unit = UNITS[i];
        let size = bytes / 1024_f64.powi(i as i32);

        if i == 0 {
            return write!(f, "{size}{unit}");
        }

        f.pad(&format!("{:.2}{:}", size, unit))
    }
}

pub struct DiskBytes<T: Copy + Into<u128>>(pub T);

impl<T: Copy + Into<u128>> std::fmt::Display for DiskBytes<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const UNITS: [&str; 7] = ["B", "KB", "MB", "GB", "TB", "PB", "EB"];

        let bytes = self.0.into() as f64;
        let i = ((bytes.log2() / 10.0) as usize).min(UNITS.len() - 1);
        let unit = UNITS[i];
        let size = bytes / 1000_f64.powi(i as i32);

        if i == 0 {
            return write!(f, "{size}{unit}");
        }

        f.pad(&format!("{:.2}{:}", size, unit))
    }
}

pub struct FmtTime<T: Copy + Into<u64>>(pub T);

impl<T: Copy + Into<u64>> std::fmt::Display for FmtTime<T> {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const UNITS: [&str; 3] = ["sec", "min", "hour"];
        let time = self.0.into() as f64;
        let i = (time.log(60.0) as usize).min(UNITS.len() - 1);
        let size = time / 60_f64.powi(i as i32);
        let pri;
        match i {
            2 => {
                let day = (size / 24.0) as usize;
                if day > 0 {
                    pri = format!("{day}day{}hour", (size as usize) - day*24);
                } else {
                    let min = size % 1.0 * 60.0;
                    pri = format!("{}hour{}min", size as usize, min as usize);
                }   
            }
            1 => {
                let sec = size % 1.0 * 60.0;
                pri = format!("{}min{}sec", size as usize, sec as usize);
            }
            0 => {
                pri = format!("{}sec", size as usize);
            }
            _ => todo!()
        }
        f.pad(&pri)
   } 
}

pub fn from_osstring(cmd: &[OsString]) -> String {
    cmd.join(OsStr::new(""))
        .to_string_lossy()
        .trim()
        .to_string()
}

pub fn command_runs(cmds: &[&[&str]]) -> anyhow::Result<String> {
    let mut child: Option<std::process::Child> = None;
    if cmds.len() > 1 {
        for args in &cmds[..cmds.len() - 1] {
            let len = args.len();
            if len == 0 {
                continue;
            }
            let mut cmd = Command::new(args[0]);
            if len > 1 {
                cmd.args(&args[1..]);
            }
            if let Some(before) = child {
                cmd.stdin(Stdio::from(before.stdout.unwrap()));
            }
            child = Some(cmd.stdout(Stdio::piped()).spawn()?);
        }
    }
    let last = cmds[cmds.len() - 1];
    let mut cmd = Command::new(last[0]);
    cmd.args(&last[1..]);
    if let Some(before) = child {
        cmd.stdin(before.stdout.unwrap());
    }

    Ok(String::from_utf8(
        cmd.stdout(Stdio::piped()).output()?.stdout,
    )?)
}
