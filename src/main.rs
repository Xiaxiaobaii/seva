use seva::ui::{art, build::Tui};

fn main() {
    art::init_art();
    let mut app = seva::App::new().expect("Create App Error");
    let terminal: Tui = ratatui::init();

    if let Err(e) = app.run(terminal) {
        eprintln!("{e}");
    };
}
