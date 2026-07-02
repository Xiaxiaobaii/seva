use criterion::{BatchSize, Bencher, Criterion, criterion_group, criterion_main};
use ratatui::{buffer::Buffer, layout::Rect};
use seva::{
    App,
    client::stream::{
        ClientState::{Info, Main, Trend},
        ui_state,
    },
    ui::art::init_art,
};

fn rectbench(bencher: &mut Bencher, app: &mut App, size: Rect) {
    let mut buffer = Buffer::empty(size);

    bencher.iter_batched(
        || {},
        |_| {
            app.flash().unwrap();
            ui_state(app, size, &mut buffer);
        },
        BatchSize::SmallInput,
    );
}

fn appflash_benchmark(c: &mut Criterion) {
    let mut app = seva::App::new().expect("Create App Error");
    c.bench_function("appflash", |b| {
        b.iter(|| {
            app.flash().unwrap();
        })
    });
}

fn full_benchmark(c: &mut Criterion) {
    let size = Rect::new(0, 0, 440, 150);
    let mut app = seva::App::new().expect("Create App Error");
    init_art();
    c.bench_function("full_main", |b| {
        app.state = Main;
        rectbench(b, &mut app, size)
    });
    c.bench_function("full_trend", |b| {
        app.state = Trend;
        rectbench(b, &mut app, size)
    });
    c.bench_function("full_info", |b| {
        app.state = Info;
        rectbench(b, &mut app, size)
    });
}

criterion_group!(benches, appflash_benchmark);
criterion_group!(fullbench, full_benchmark);
criterion_main!(benches, fullbench);
