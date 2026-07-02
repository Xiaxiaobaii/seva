use std::{collections::HashMap, io, rc::Rc, time::Duration};

use crossterm::event::{self, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    style::Style,
    symbols::{self, line::DOUBLE_VERTICAL, merge::MergeStrategy},
    text::Line,
    widgets::{LineGauge, Paragraph},
};
use sysinfo::{
    Disks, Motherboard, Networks, Pid, Process, ProcessRefreshKind, ProcessesToUpdate, System,
};

use crate::{
    client::{
        server::{self, Serve},
        system::{Config, SystemLine, command_runs, from_osstring, sec_to_time},
    },
    sys::{Disk, take_sys_disk},
    ui::build::{Tui, normal_block, set_alert},
};
use crate::{
    client::{stream::ClientState, system::HumanBytes},
    sys::get_gpu,
};

pub mod client;
pub mod sys;
pub mod ui;

const APP_VERSION: &'static str = "1.1.4-pre";

unsafe impl Send for App {}
pub struct App {
    pub state: ClientState,
    pub exit: bool,
    pub sys: System,
    pub extend: Extend,
    pub sys_line: SystemLine,
    pub err: Option<anyhow::Error>,
    pub config: Config,
    pub service: Serve,
    pub network: Networks,
    pub formats: Format,
    pub disks: Vec<Disk>,
}

impl App {
    pub fn new() -> Result<App, anyhow::Error> {
        let config = Config::new();
        Ok(App {
            state: ClientState::Main,
            exit: false,
            sys: System::new_all(),
            sys_line: SystemLine::new(),
            err: None,
            extend: Extend::new()?,
            config: config.clone(),
            service: Serve::new(config.services),
            network: Networks::new_with_refreshed_list(),
            formats: Format::new(),
            disks: take_sys_disk().unwrap(),
        }
        .once())
    }

    fn once(mut self) -> Self {
        let gpu = get_gpu().unwrap_or_default();
        self.formats.tab = Rc::new(
            Paragraph::new(format!("Welcome to SeVA {APP_VERSION}!   Press 'Q' to quit SEVA"))
                .alignment(ratatui::layout::Alignment::Center)
                .block(normal_block("SEVA Control")),
        );

        self.formats.os_message_format = format!(
            "os: {}-{}\ncpu name: {}\nMotherboard: {}\nkernel version: {}\nhost name: {}\nup time: {}\n{}\n",
            System::name().unwrap_or_default(),
            System::os_version().unwrap_or(String::from("Unknown os version")),
            self.sys.cpus()[0].brand(),
            Motherboard::new()
                .map(|x| x.name().unwrap_or(String::new()))
                .unwrap_or("".to_string()),
            System::kernel_version().unwrap_or_default(),
            System::host_name().unwrap_or(String::from("linux")),
            sec_to_time(System::uptime()),
            self.extend.package_text,
        );

        for (i, dev) in gpu.iter().enumerate() {
            if let Some(name) = &dev.device_name {
                self.formats.os_message_format += &format!("gpu {}: {}\n", i, name);
            }
        }

        self.flash().unwrap();

        self
    }

    pub fn destory(self) -> io::Result<()> {
        Ok(())
    }

    pub fn flash(&mut self) -> anyhow::Result<()> {
        if self.extend.space {
            return Ok(());
        }
        self.extend.disks.iter_mut().for_each(|disk| {
            disk.refresh();
        });
        self.network.refresh(true);
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        self.merge_process();
        self.sys_line.swap_data.force_queue(
            format!(
                "{:.2}",
                (self.sys.used_swap() as f64 / self.sys.total_swap() as f64) * 100.0
            )
            .parse::<f64>()?,
        );
        self.sys_line
            .cpu_data
            .force_queue(self.sys.global_cpu_usage().into());
        let us_memory = self.sys.used_memory() as f64;
        let to_memory = self.sys.total_memory() as f64;
        self.sys_line
            .memory_data
            .force_queue(format!("{:.2}", (us_memory / to_memory) * 100.0).parse::<f64>()?);
        self.merge_ui();
        Ok(())
    }

    pub fn merge_process(&mut self) {
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory()
                .with_exe(sysinfo::UpdateKind::OnlyIfNotSet)
                .without_tasks(),
        );
        let mut memory_verify: HashMap<String, MutProcess> = HashMap::new();
        for item in self.sys.processes().values() {
            if item.thread_kind().is_some() {
                continue;
            }
            let item = MutProcess::from_process(item);
            let mem = item.memory;
            let cpu_usage = item.cpu_usage;

            if let Some(o2) = memory_verify.get_mut(&item.cmd) {
                o2.cpu_usage += cpu_usage;
                o2.memory += mem;
                continue;
            } else {
                memory_verify.insert(item.cmd.clone(), item);
            }
        }

        self.extend.processes = memory_verify.into_values().collect();

        if self.extend.trend_sort == "mem" {
            self.extend
                .processes
                .sort_by(|k, v| v.memory.cmp(&k.memory));
        } else {
            self.extend
                .processes
                .sort_by(|k, v| v.cpu_usage.total_cmp(&k.cpu_usage));
        }
    }

    pub fn merge_ui(&mut self) {
        let blue_style = Style::new().blue().on_black().bold();
        self.formats.cpu_line = Rc::new(
            LineGauge::default()
                .block(normal_block("cpu").merge_borders(MergeStrategy::Exact))
                .filled_style(blue_style)
                .filled_symbol(DOUBLE_VERTICAL)
                .unfilled_symbol(symbols::line::DOUBLE_VERTICAL)
                .label(Line::default())
                .ratio(self.sys.global_cpu_usage() as f64 / 100.0),
        );

        self.formats.mem_line = Rc::new(
            LineGauge::default()
                .block(normal_block("memory").merge_borders(MergeStrategy::Exact))
                .filled_style(blue_style)
                .filled_symbol(symbols::line::DOUBLE_VERTICAL)
                .unfilled_symbol(symbols::line::DOUBLE_VERTICAL)
                .label(Line::default())
                .ratio(self.sys.used_memory() as f64 / self.sys.total_memory() as f64),
        );

        self.formats.swap_line = Rc::new(
            LineGauge::default()
                .block(normal_block("swap").merge_borders(MergeStrategy::Exact))
                .filled_style(blue_style)
                .filled_symbol(symbols::line::DOUBLE_VERTICAL)
                .unfilled_symbol(symbols::line::DOUBLE_VERTICAL)
                .label(Line::default())
                .ratio(self.sys.used_swap() as f64 / self.sys.total_swap() as f64),
        );
            
        self.formats.disk_text = self.extend.disks.iter().fold(String::new(), |acc, disk| {
            let total_space = disk.total_space();
            if total_space < 8 * 1024 * 1024 * 1024 {
                return acc; 
            }
            acc + &format!(
                "Disk Name: {:?}\n   file system: {:?}\n   used/total: {}/ {}\n   write/read: {}/ {}\n\n",
                disk.name(),
                disk.file_system(),
                HumanBytes(total_space - disk.available_space()),
                HumanBytes(total_space),
                HumanBytes(disk.usage().written_bytes),
                HumanBytes(disk.usage().read_bytes)
            )
        });
    }

    pub fn run(&mut self, mut terminal: Tui) -> anyhow::Result<()> {
        
        //let mut reader = EventStream::new();

        loop {
            //let next = reader.next().fuse();
            if event::poll(Duration::from_secs(1))? {
                if let event::Event::Key(key) = event::read()? {
                    handle_key(self, key)?;
                }
            }
            self.flash()?;
            if self.exit {
                break;
            }

            terminal.draw(|frame| {
                let area = frame.area();
                let buf = frame.buffer_mut();
                crate::client::stream::ui_state(self, area, buf);
                if let Some(err) = &self.err {
                    set_alert(area, buf, &err.to_string());
                }
            })?;
        }
        ratatui::restore();
        terminal.show_cursor()?;

        Ok(())
    }
}

#[derive(Default)]
pub struct Format {
    os_message_format: String,
    tab: Rc<Paragraph<'static>>,
    cpu_line: Rc<LineGauge<'static>>,
    mem_line: Rc<LineGauge<'static>>,
    swap_line: Rc<LineGauge<'static>>,
    disk_text: String,
}

impl Format {
    pub fn new() -> Format {
        Format {
            ..Default::default()
        }
    }
}

#[derive(Default)]
pub struct Extend {
    pub package_text: String,
    pub trend_sort: String,
    pub space: bool,
    pub disks: Disks,
    pub processes: Vec<MutProcess>,
}

impl Extend {
    pub fn new() -> anyhow::Result<Extend> {
        let mut result = Extend {
            package_text: String::from("packages: "),
            ..Default::default()
        };
        if let Ok(n) = command_runs(&[&["dpkg", "-l"], &["grep", "ii"], &["wc", "-l"]]) {
            result.package_text += &format!("{} (dpkg), ", n.trim());
        }
        if let Ok(n) = command_runs(&[&["pacman", "-Q"], &["wc", "-l"]]) {
            result.package_text += &format!("{} (pacman), ", n.trim());
        }
        result.package_text = result.package_text.trim_end_matches(", ").to_string();
        result.disks = Disks::new_with_refreshed_list();
        Ok(result)
    }
}

pub fn handle_key(main: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    if key.kind == KeyEventKind::Press {
        if let Some(_) = &main.err
            && key.code == KeyCode::Enter
        {
            main.err = None;
        }
        if key.code == KeyCode::Tab {
            crate::client::stream::reset_state(&mut main.state);
        } else if key.code == KeyCode::Enter {
            main.flash()?;
        }

        let state = &main.state;
        if *state == ClientState::Serve {
            server::main_event(main, key)?;
        } else if *state == ClientState::Trend {
            if key.code == KeyCode::Char('m') {
                main.extend.trend_sort = String::from("mem")
            } else if key.code == KeyCode::Char(' ') {
                main.extend.space = !main.extend.space;
            } else if key.code == KeyCode::Char('c') {
                main.extend.trend_sort = String::from("")
            }
        }
        if key.code == KeyCode::Char('q') || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)) {
            main.exit = true;
        }
    }
    Ok(())
}

pub struct MutProcess {
    pub pid: Pid,
    pub cmd: String,
    pub cpu_usage: f32,
    pub memory: u64,
    pub virtual_memory: u64,
    pub run_time: u64,
    pub name: String,
}

impl MutProcess {
    pub fn from_process(process: &Process) -> MutProcess {
        let cmd;
        if let Some(pat) = process.exe() {
            cmd = pat.to_string_lossy().to_string();
        } else {
            cmd = from_osstring(process.cmd());
        }

        MutProcess {
            pid: process.pid(),
            cmd,
            cpu_usage: process.cpu_usage(),
            memory: process.memory(),
            virtual_memory: process.virtual_memory(),
            run_time: process.run_time(),
            name: process.name().to_string_lossy().to_string(),
        }
    }
}
