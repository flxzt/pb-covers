use anyhow::anyhow;
use embedded_graphics::pixelcolor::Gray8;
use embedded_graphics::prelude::*;
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use guillotine::{DirectTarget, FrameStorage, Ui};
use pb_covers::state::{App, PB_TOUCHLUX3_SCREEN_SIZE};
use pb_covers::ui::CoverImageNode;
use sdl2::keyboard::Keycode;
use std::env;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let search_dir: PathBuf = PathBuf::from(env::args().nth(1).ok_or(anyhow!(
        "Usage: pb-covers-simulator <books search directory>"
    ))?);
    if !search_dir.exists() {
        return Err(anyhow!(
            "Search directory '{}' does not exist",
            search_dir.display()
        ));
    }
    eprintln!("Searching for books in '{}'", search_dir.display());

    let display = SimulatorDisplay::<Gray8>::new(PB_TOUCHLUX3_SCREEN_SIZE);
    let storage = FrameStorage::<Gray8, 64, 1024, CoverImageNode>::default();
    let theme = guillotine::Theme {
        background: Gray8::WHITE,
        foreground: Gray8::BLACK,
    };
    let mut ui = Ui::with_theme(DirectTarget::new(display), storage, theme);
    let mut app = App::new(search_dir, PB_TOUCHLUX3_SCREEN_SIZE);
    let output_settings = OutputSettingsBuilder::new().scale(1).build();
    let title = env!("CARGO_BIN_NAME");
    let mut quit = false;
    let mut window = Window::new(title, &output_settings);
    window.set_max_fps(60);
    ui.render(&app).unwrap();
    window.update(ui.display());

    'running: while !quit {
        for event in window.events() {
            match event {
                SimulatorEvent::Quit => break 'running,
                SimulatorEvent::KeyDown { keycode, .. } => match keycode {
                    Keycode::Left => {
                        app.pos_cycle_backward();
                    }
                    Keycode::Right => {
                        app.pos_cycle_forward();
                    }
                    Keycode::Space => {
                        app.toggle_overlay();
                    }
                    Keycode::PageUp => {
                        app.orientation_cycle_backward();
                    }
                    Keycode::PageDown => {
                        app.orientation_cycle_forward();
                    }
                    Keycode::Escape => {
                        quit = true;
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        ui.render(&app).unwrap();
        window.update(ui.display());
    }

    Ok(())
}
