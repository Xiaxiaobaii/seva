use queue::Queue;
use ratatui::{
    Terminal, buffer::Buffer, layout::{Constraint, Flex, Layout, Margin, Rect, Spacing}, style::{Style}, symbols::{Marker, border, line::{VERTICAL}, merge::MergeStrategy}, text::{Line}, widgets::{
        Axis, Block, Chart, Clear, Dataset, GraphType, LineGauge, List, Padding, Paragraph, Widget, Wrap,
    },
};
use std::{io, ops::Deref, vec};
use sysinfo::{Motherboard, System};

use crate::{
    App,
    client::system::{DiskBytes, FmtTime, HumanBytes, command_runs},
    sys::{ModernDmiDecodedData, decode_dmi},
    ui::layout::{info_layout, main_layout, trend_layout},
};

pub type Tui = Terminal<ratatui::prelude::CrosstermBackend<io::Stdout>>;

pub fn info_ui(app: &crate::App, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
    let cache;
    if let Ok(c) = command_runs(&[&["lscpu"], &["grep", "^L"]]) {
        cache = c.replace(" ", "");
    } else {
        cache = String::from("无法获取缓存信息");
    }

    let (hello, product, cache_rect, disk, memory) = info_layout(area, buf, cache.lines().collect::<Vec<&str>>().len());

    let cpubrand = app.sys.cpus()[0].brand();
    let dmi = decode_dmi();

    Paragraph::new(format!(
        "hello? {}",
        System::host_name().unwrap_or(String::from("linux"))
    ))
    .block(normal_block(""))
    .render(hello, buf);
    let dmi = dmi.map(|dmi| ModernDmiDecodedData::from_dmidecoded(&dmi).unwrap());

    if let Some(mother) = Motherboard::new() {
        let mut text = format!(
            "name: {}{}\ncpu name: {}\ncpu arch: {}\ncpu logic number: {}\n",
            mother.vendor_name().unwrap_or_default(),
            mother.name().unwrap_or_default(),
            System::cpu_arch(),
            cpubrand,
            app.sys.cpus().len()
        );

        if let Ok(dmi) = dmi.as_ref() {
            text = format!(
                "product name: {}\nserial number: {}\ncpu name: {}\ncpu logic number: {}\n",
                dmi.system.product_name,
                dmi.system.serial_number,
                cpubrand,
                app.sys.cpus().len(),
            );
            if let Some(family) = &dmi.system.family {
                text += &format!("system family: {}\n", family)
            }
            text += &format!(
                "system max memory: {}\n",
                HumanBytes(dmi.memory.max_capacity)
            );
            text += &format!("physical memory slot count: {}\n", dmi.memory.max_slots);
        } else {
            text += "以root权限启动以查看更多信息";
        }
        Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .block(normal_block("product"))
            .render(product, buf);
        Paragraph::new(cache)
            .wrap(Wrap { trim: true })
            .block(normal_block("cache"))
            .render(cache_rect, buf);
    }

    let disk_test = app
        .disks
        .iter()
        .fold(String::new(), |mut acc, f: &crate::sys::Disk| {
            if let Some(name) = &f.disk_name {
                acc += &format!(
                    "{}({}) {}",
                    name.trim(),
                    if f.ssd { "SSD" } else { "HDD" },
                    f.format_size,
                );
                if let Some(serial) = &f.serial {
                    acc += &format!(" - {}", serial);
                }

                if let Some(nvmespc) = &f.nvmespc
                    && let Some(firmware_version) = &f.firmware_version
                {
                    acc += &format!(
                        "\n  NVME spc: {}, firmware version: {}",
                        nvmespc, firmware_version
                    );
                }
                if let Some(smartlog) = &f.smartlog {
                    acc += &format!(
                        "\n  media error: {}, unit read/written: {}/{}",
                        smartlog.media_errors(),
                        DiskBytes(smartlog.data_units_read() * 512 * 1000),
                        DiskBytes(smartlog.data_units_written() * 512 * 1000)
                    );
                    if let Some(speed) = &f.format_pcie {
                        acc += &format!(
                            "\n  temperature: {} C, Percentage Used: {}%, {}",
                            smartlog.temperature_celsius(),
                            smartlog.percentage_used(),
                            speed
                        );
                    } else {
                        acc += &format!(
                            "\n  temperature: {} C, Percentage Used: {}%",
                            smartlog.temperature_celsius(),
                            smartlog.percentage_used(),
                        );
                    }
                }

                acc += "\n"
            }
            acc
        });
    Paragraph::new(disk_test)
        .block(normal_block("disk").merge_borders(MergeStrategy::Exact))
        .render(disk, buf);

    let mut mem_text = String::new();
    if let Ok(memory) = dmi.map(|dmi| dmi.memory) {
        let mut i = 0;
        memory.devices.iter().for_each(|x| {
            mem_text += &format!("slot{i}: \n  内存类型: {:?}\n  内存大小: {}\n  内存型号: {}\n  内存技术: {:?}\n  内存制造商: {}\n  内存速度: {}MT/s\n  内存配置速度: {}MT/s\n  内存最小电压: {}mV\n  内存最大电压: {}mV\n  内存配置电压: {}mV\n",
            x.memory_type, HumanBytes(x.size), x.part_number, x.trchnology, x.manufacturer, x.max_speed, x.configured_speed, x.min_voltage, x.max_voltage, x.configured_voltage);
            i += 1;
        });
    } else {
        mem_text = String::from("以root权限启动以查看内存信息");
    }
    Paragraph::new(mem_text)
        .block(normal_block("mem"))
        .render(memory, buf);
}

fn once_cpu(id: i32, usage: f32, area: Rect, buf: &mut Buffer) {
    LineGauge::default()
        .filled_style(Style::new().blue().on_black().bold())
        .filled_symbol(VERTICAL)
        .unfilled_symbol(VERTICAL)
        .label(Line::default())
        .ratio(usage as f64 / 100.0)
        .render(area, buf);
    Paragraph::new(format!(
        "cpu{id} {:.2}%",
        usage
    ))
    .alignment(ratatui::layout::HorizontalAlignment::Right)
    .render(area, buf);
}

pub fn trend_ui(
    app: &crate::App,
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
) {
    let (trend, mut disk, process) = trend_layout(area, buf);

    let pc = PercentageChart::set(
        String::from("trend"),
        vec![
            app.sys_line.memory_data.clone(),
            app.sys_line.swap_data.clone(),
            app.sys_line.cpu_data.clone(),
        ],
        vec![
            String::from("mem"),
            String::from("swap"),
            String::from("cpu"),
        ],
        vec![
            Style::new().red(),
            Style::new().green(), 
            Style::new().cyan(),
        ],
    );
    pc.build(trend, buf);

    normal_block("").render(disk, buf);
    let margin = Margin::new(1, 1);
    disk = disk.inner(margin);

    let mut cpu_iter = app.sys.cpus().iter();

    let mut disks: [Rect; 3] = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1), Constraint::Fill(1)]).areas(disk);
    
    let mut cursor = 0;
    let mut i = 1;
    
    while let Some(cpu) = cpu_iter.next() {
        if cursor == disks.len() {
            cursor = 0;
        }
        let disk = disks.get_mut(cursor).unwrap();
        let [disk_c, disk_f] = Layout::vertical([Constraint::Length(2), Constraint::Fill(1)]).areas(*disk);
        once_cpu(i, cpu.cpu_usage(), disk_c, buf);
        *disk = disk_f;
        cursor+=1;
        i+=1;

    }

    // let item1 = Text::from(app.formats.disk_text.as_str())
    //     .centered()
    //     .bg(Color::White)
    //     .fg(Color::White);
    // List::new(item1)
    //     .block(normal_block("Disk"))
    //     .render(disk, buf);

    let [process_top, process] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(5)]).areas(process);

    Paragraph::new("   PID      %CPU        MEM           TIME             CMD")
        .render(process_top, buf);

    let items = app.extend.processes.iter().map(|process| {
        format!(
            "{:<9}{:<12}{:<14}{:<17}{}",
            process.pid.as_u32(),
            format!("{:.2}%", process.cpu_usage),
            HumanBytes(process.memory),
            FmtTime(process.run_time),
            process.cmd
        )
    });

    List::new(items)
        .block(normal_block("process"))
        .render(process, buf);
}

pub fn main_ui(app: &crate::App, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
    let (tabs, line, os, network, process) = main_layout(area, buf);

    app.formats.tab.deref().render(tabs, buf);

    let [cpu, memory, swap] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(4),
    ])
    .spacing(Spacing::Overlap(1))
    .areas(line);

    let [process_top, process] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(5)]).areas(process);

    app.formats.mem_line.deref().render(memory, buf);
    app.formats.swap_line.deref().render(swap, buf);
    app.formats.cpu_line.deref().render(cpu, buf);

    Paragraph::new(format!(
        "{:<5}/{:<5} ",
        HumanBytes(app.sys.used_memory()),
        HumanBytes(app.sys.total_memory())
    ))
    .block(normal_block("memory").merge_borders(MergeStrategy::Exact))
    .alignment(ratatui::layout::HorizontalAlignment::Right)
    .render(memory, buf);

    Paragraph::new(format!(
        "{:<5}/{:<5} ",
        HumanBytes(app.sys.used_swap()),
        HumanBytes(app.sys.total_swap())
    ))
    .block(normal_block("swap").merge_borders(MergeStrategy::Exact))
    .alignment(ratatui::layout::HorizontalAlignment::Right)
    .render(swap, buf);

    Paragraph::new(format!("{:.2}% ", app.sys.global_cpu_usage()))
        .block(normal_block("cpu").merge_borders(MergeStrategy::Exact))
        .alignment(ratatui::layout::HorizontalAlignment::Right)
        .render(cpu, buf);

    Paragraph::new(app.formats.os_message_format.clone())
        .wrap(Wrap { trim: true })
        .block(normal_block("os"))
        .render(os, buf);

    let mut items: Vec<String> = vec![];
    for (pid, item) in &app.network {
        items.push(format!(
            "{}: {:<5} (Down) / {:<5} (Up)",
            pid,
            HumanBytes(item.total_received()),
            HumanBytes(item.total_transmitted())
        ));
    }

    List::new(items)
        .block(normal_block("network"))
        .render(network, buf);

    let (a, b) = if buf.area.as_size().height > 25 {
        short_process(app)
    } else {
        long_process(app)
    };
    a.render(process_top, buf);
    b.render(process, buf);
}

pub fn long_process(app: &App) -> (Paragraph<'static>, List<'static>) {
    let items = app.extend.processes.iter().map(|process| {
        format!(
            "{:<9}{:<12}{:<14}{:<17}{}",
            process.pid.as_u32(),
            format!("{:.2}%", process.cpu_usage),
            HumanBytes(process.memory),
            FmtTime(process.run_time),
            process.cmd
        )
    });
    (
        Paragraph::new("   PID      %CPU        MEM           TIME             CMD"),
        List::new(items).block(normal_block("process")),
    )
}

pub fn short_process(app: &App) -> (Paragraph<'static>, List<'static>) {
    let items = app.extend.processes.iter().map(|x| {
        format!(
            "{:<9}{:<17}{:<10}{}",
            x.pid.as_u32(),
            x.name,
            format!("{:.2}%", x.cpu_usage),
            HumanBytes(x.memory),
        )
    });
    (
        Paragraph::new("    PID      NAME            %CPU       MEM"),
        List::new(items).block(normal_block("process")),
    )
}

pub fn normal_block(name: &str) -> Block<'_> {
    Block::bordered()
        .title(name)
        .padding(Padding::horizontal(2))
        .border_style(Style::default().fg(ratatui::style::Color::Yellow))
        .border_set(border::THICK)
}

pub struct PercentageChart {
    data: Vec<Queue<f64>>,
    data_name: Vec<String>,
    name: String,
    colors: Vec<Style>,
    live_vec: Vec<Vec<(f64, f64)>>,
}

impl PercentageChart {
    pub fn set(
        name: String,
        data: Vec<Queue<f64>>,
        data_name: Vec<String>,
        colors: Vec<Style>,
    ) -> PercentageChart {
        PercentageChart {
            data,
            data_name,
            name,
            colors,
            live_vec: vec![],
        }
    }
    pub fn build(mut self, area: Rect, buf: &mut Buffer) {
        let mut top: f64 = 0.0;
        let mut min: f64 = 100.0;
        let mut capacity: f64 = self.data[0].capacity().unwrap() as f64;
        let mut re_vec = vec![];
        self.data.into_iter().for_each(|data| {
            if capacity > data.capacity().unwrap() as f64 {
                capacity = data.capacity().unwrap() as f64;
            }
            let mut live_vec = vec![];

            for (i, v) in data.vec().iter().enumerate() {
                let v = *v;
                if v > top {
                    top = v + 1.0;
                }
                if v < min && v >= 0.4 {
                    min = v - 0.4;
                } else if v < min {
                    min = v;
                }
                live_vec.push((i as f64, v));
            }
            self.live_vec.push(live_vec);
        });
        for (ref_i, point) in self.live_vec.iter().enumerate() {
            re_vec.push(
                Dataset::default()
                    .name(self.data_name[ref_i].clone())
                    .marker(Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(self.colors[ref_i])
                    .data(point),
            );
        }

        top = format!("{:.2}", top).parse::<f64>().unwrap();
        min = format!("{:.2}", min).parse::<f64>().unwrap();
        let mid = format!("{:.2}", (top + min) / 2.0).parse::<f64>().unwrap();
        let tmid = format!("{:.2}", (top + mid) / 2.0).parse::<f64>().unwrap();
        let mmid = format!("{:.2}", (min + mid) / 2.0).parse::<f64>().unwrap();

        Chart::new(re_vec)
            .x_axis(Axis::default().bounds([0.0, capacity]))
            .y_axis(Axis::default().bounds([min, top]).labels([
                min.to_string() + "%",
                mmid.to_string() + "%",
                mid.to_string() + "%",
                tmid.to_string() + "%",
                top.to_string() + "%",
            ]))
            .hidden_legend_constraints((Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)))
            .block(normal_block(&self.name))
            .render(area, buf);
    }
}

pub fn create_pop(x: u16, y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

pub fn set_alert(area: Rect, buf: &mut Buffer, text: &str) {
    Clear.render(create_pop(70, 10, area), buf);
    Paragraph::new(text)
        .block(normal_block("alert"))
        .render(create_pop(60, 10, area), buf);
}

pub struct Alert {
    x: u16,
    y: u16,
    text: String,
    title: String,
}

impl Alert {
    pub fn new(x: u16, y: u16, text: String) -> Alert {
        Alert {
            x,
            y,
            text,
            title: String::new(),
        }
    }

    pub fn set_title(mut self, title: String) -> Self {
        self.title = title;
        self
    }

    pub fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(create_pop(self.x, self.y, area), buf);
        Paragraph::new(self.text)
            .block(normal_block(&self.title))
            .render(create_pop(self.x, self.y, area), buf);
    }
}
