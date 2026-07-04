use ratatui::layout::{Constraint, Layout, Rect};

use crate::ui::art;

pub fn main_layout(
    area: Rect,
    buf: &mut ratatui::prelude::Buffer,
    network_size: usize,
    os_size: usize
) -> (Rect, Rect, Rect, Rect, Rect) {
    let [tabs, main] = Layout::vertical([Constraint::Length(3), Constraint::Fill(0)]).areas(area);
    // 25 + 2 + 1
    if buf.area.as_size().height > 28+network_size as u16 {
        let [art_network, mem_os_process] =
            Layout::horizontal([Constraint::Length(53), Constraint::Fill(1)]).areas(main);

        let [art, network] =
            Layout::vertical([Constraint::Length(24), Constraint::Fill(1)]).areas(art_network);

        let [mem_os, process] =
            Layout::vertical([Constraint::Length(7+os_size as u16), Constraint::Fill(1)]).areas(mem_os_process);

        let [line, os] =
            Layout::vertical([Constraint::Length(7), Constraint::Length(os_size as u16)]).areas(mem_os);

        art::render_logo(art, buf);

        (tabs, line, os, network, process)
    } else {
        let [top, process] =
            Layout::vertical([Constraint::Fill(1), Constraint::Fill(1)]).areas(main);
        let [cpu_mem_os, os] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(top);

        let [line, network] =
            Layout::vertical([Constraint::Max(7), Constraint::Fill(0)]).areas(cpu_mem_os);

        (tabs, line, os, network, process)
    }
}

pub fn trend_layout(area: Rect, _buf: &mut ratatui::prelude::Buffer) -> (Rect, Rect, Rect) {
    let [trend_disk, process] =
        Layout::vertical([Constraint::Fill(1), Constraint::Fill(1)]).areas(area);

    let [trend, disk] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(trend_disk);

    (trend, disk, process)
}

pub fn info_layout(
    area: Rect,
    _buf: &mut ratatui::prelude::Buffer,
    cache_size: usize,
) -> (Rect, Rect, Rect, Rect, Rect) {
    let [hello, area] = Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(area);
    let [hello, _] = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(3)]).areas(hello);
    let [motherboard, disk_memory] = Layout::horizontal([
        Constraint::Fill(2),
        Constraint::Fill(3),
    ])
    .areas(area);

    let [product, cache] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(cache_size as u16+2)])
            .areas(motherboard);

    let [memory, disk] =
        Layout::vertical([Constraint::Fill(1), Constraint::Fill(1)]).areas(disk_memory);


    (hello, product, cache, disk, memory)
}
