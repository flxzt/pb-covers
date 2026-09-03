use core::time::Duration;
use embedded_graphics::prelude::*;
use embedded_graphics_core::pixelcolor::Gray8;
use guillotine::{DirectTarget, FrameStorage, Ui};
use inkview::event::Key;
use inkview_eg::InkviewDisplay;
use pb_covers::state::{App, PB_INKPAD4_SCREEN_SIZE};
use pb_covers::ui::CoverImageNode;
use std::path::PathBuf;
use std::sync::{LazyLock, mpsc};
use std::{env, thread};

static DEFAULT_SEARCH_PATH: &str = "/mnt/ext1/Literature";
const INIT_SCREEN_SIZE: Size = PB_INKPAD4_SCREEN_SIZE;

#[derive(Debug, Clone)]
enum CoversEvent {
    InkviewEvent(inkview::Event),
    CoverCycleForward,
}

fn main() -> anyhow::Result<()> {
    let (event_tx, event_rx) = mpsc::channel::<CoversEvent>();
    let iv = Box::leak(Box::new(inkview::load())) as &_;

    let search_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or(PathBuf::from(DEFAULT_SEARCH_PATH));
    let cycle_duration: Duration = env::args()
        .nth(2)
        .map(|s| Duration::from_secs(s.parse::<u64>().expect("Not a valid number")))
        .unwrap_or(Duration::from_secs(60));

    thread::spawn(move || {
        let mut app = App::new(search_dir, INIT_SCREEN_SIZE);
        let mut ui = LazyLock::new(|| {
            let mut display = InkviewDisplay::new(iv);
            display.clear(Gray8::WHITE);
            display.flush();
            let storage = FrameStorage::<Gray8, 64, 1024, CoverImageNode>::default();
            let theme = guillotine::Theme {
                background: Gray8::WHITE,
                foreground: Gray8::BLACK,
            };

            Ui::with_theme(DirectTarget::new(display), storage, theme)
        });

        loop {
            let mut render = false;
            let event = match event_rx.recv() {
                Ok(e) => e,
                Err(err) => {
                    eprintln!("Receiving inkview event failed, Err: {err:?}");
                    break;
                }
            };
            eprintln!("Received inkview event {event:?}");
            match event {
                CoversEvent::InkviewEvent(inkview::Event::Init) => {
                    let ui = LazyLock::force_mut(&mut ui);
                    app.update_screen_size(ui.display().size());
                    render = true;
                }
                CoversEvent::InkviewEvent(inkview::Event::Show)
                | CoversEvent::InkviewEvent(inkview::Event::Repaint) => {
                    render = true;
                }
                CoversEvent::InkviewEvent(inkview::Event::KeyDown { key: Key::Prev })
                | CoversEvent::InkviewEvent(inkview::Event::KeyDown { key: Key::Prev2 }) => {
                    if app.overlay() {
                        app.orientation_cycle_backward();
                    } else {
                        app.pos_cycle_backward();
                    }
                    render = true;
                }
                CoversEvent::InkviewEvent(inkview::Event::KeyDown { key: Key::Next })
                | CoversEvent::InkviewEvent(inkview::Event::KeyDown { key: Key::Next2 }) => {
                    if app.overlay() {
                        app.orientation_cycle_forward();
                    } else {
                        app.pos_cycle_forward();
                    }
                    render = true;
                }
                CoversEvent::InkviewEvent(inkview::Event::KeyDown { key: Key::Menu }) => {
                    app.toggle_overlay();
                    render = true;
                }
                CoversEvent::InkviewEvent(inkview::Event::KeyDown { .. })
                | CoversEvent::InkviewEvent(inkview::Event::Exit) => break,
                CoversEvent::CoverCycleForward => {
                    app.pos_cycle_forward();
                    render = true;
                }
                _ => {}
            }
            #[allow(clippy::collapsible_if)]
            if render && let Some(ui) = LazyLock::get_mut(&mut ui) {
                if let Err(err) = ui.render(&app) {
                    eprintln!("Unable to render app: {err}");
                }
                ui.display_mut().flush();
            }
        }

        unsafe { iv.CloseApp() }
    });

    let event_tx_c = event_tx.clone();
    thread::spawn(move || {
        loop {
            thread::sleep(cycle_duration);
            if let Err(err) = event_tx_c.send(CoversEvent::CoverCycleForward) {
                eprintln!("Sending cycle cover event failed, Err: {err:?}");
            }
        }
    });

    inkview::iv_main(iv, move |event| {
        if let Err(err) = event_tx.send(CoversEvent::InkviewEvent(event)) {
            eprintln!("Sending inkview event failed, Err: {err:?}");
        }
        Some(())
    });

    Ok(())
}
