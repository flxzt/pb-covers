use core::time::Duration;
use embedded_graphics::image::ImageRaw;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::pixelcolor::raw::BigEndian;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyleBuilder, StyledDrawable};
use embedded_graphics::text::{Alignment, Text};
use embedded_graphics_core::pixelcolor::Gray8;
use inkview::event::Key;
use inkview_eg::InkviewDisplay;
use pb_covers::state::State;
use std::cell::OnceCell;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::{LazyLock, mpsc};
use std::{env, thread};

static DEFAULT_SEARCH_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from("/mnt/ext1/Literature"));

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
        .unwrap_or(DEFAULT_SEARCH_PATH.clone());
    let cycle_duration: Duration = env::args()
        .nth(2)
        .map(|s| Duration::from_secs(s.parse::<u64>().expect("Not a valid number")))
        .unwrap_or(Duration::from_secs(60));
    let mut state = State::new(search_dir);

    thread::spawn(move || {
        let mut display = OnceCell::new();

        loop {
            let mut render = false;
            let event = match event_rx.recv() {
                Ok(e) => e,
                Err(err) => {
                    eprintln!("Receiving inkview event failed, Err: {err:?}");
                    break;
                }
            };
            match event {
                CoversEvent::InkviewEvent(inkview::Event::Init) => {
                    // Create a new inkview display which implements [embedded_graphics_core::DrawTarget]
                    let mut iv_display = InkviewDisplay::new(iv);
                    iv_display.clear(Gray8::WHITE);
                    iv_display.flush();
                    let _ = display.set(iv_display);
                    render = true;
                }
                CoversEvent::InkviewEvent(inkview::Event::Show)
                | CoversEvent::InkviewEvent(inkview::Event::Repaint) => {
                    render = true;
                }
                CoversEvent::InkviewEvent(inkview::Event::KeyDown { key: Key::Prev })
                | CoversEvent::InkviewEvent(inkview::Event::KeyDown { key: Key::Prev2 }) => {
                    if state.overlay() {
                        state.orientation_cycle_backward();
                    } else {
                        state.pos_cycle_backward();
                    }
                    render = true;
                }
                CoversEvent::InkviewEvent(inkview::Event::KeyDown { key: Key::Next })
                | CoversEvent::InkviewEvent(inkview::Event::KeyDown { key: Key::Next2 }) => {
                    if state.overlay() {
                        state.orientation_cycle_forward();
                    } else {
                        state.pos_cycle_forward();
                    }
                    render = true;
                }
                CoversEvent::InkviewEvent(inkview::Event::KeyDown { key: Key::Menu }) => {
                    state.toggle_overlay();
                    render = true;
                }
                CoversEvent::InkviewEvent(inkview::Event::KeyDown { .. })
                | CoversEvent::InkviewEvent(inkview::Event::Exit) => break,
                CoversEvent::CoverCycleForward => {
                    state.pos_cycle_forward();
                    render = true;
                }
                _ => {}
            }

            if render && let Some(display) = display.get_mut() {
                if let Err(err) = draw_content(&mut state, display) {
                    eprintln!("Unable to draw content: {err:?}");
                };
                display.flush()
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
        // Disable inkview sleep, use our own routines.
        unsafe { iv.iv_sleepmode(0) };
        if let Err(err) = event_tx.send(CoversEvent::InkviewEvent(event)) {
            eprintln!("Sending inkview event failed, Err: {err:?}");
        }
        Some(())
    });

    Ok(())
}

fn draw_content(
    state: &mut State,
    display: &mut impl DrawTarget<Color = Gray8, Error = Infallible>,
) -> anyhow::Result<()> {
    let display_size = display.bounding_box().size;
    let text_style = MonoTextStyle::new(&FONT_10X20, Gray8::new(0x00));
    let text_bg_style = PrimitiveStyleBuilder::new()
        .stroke_color(Gray8::BLACK)
        .fill_color(Gray8::WHITE)
        .build();

    if let Some(buf) = state.cover_image_buf(display_size) {
        // draw the cover image..
        let image = ImageRaw::<Gray8, BigEndian>::new(buf.as_raw(), buf.width());
        image.draw(display);
    } else {
        // ..or a placeholder text
        display.clear(Gray8::WHITE);
        let text = "- no cover found -";
        Text::with_alignment(
            text,
            display.bounding_box().center(),
            text_style,
            Alignment::Center,
        )
        .draw(display)?;
    };

    if state.overlay() {
        let text_str = format!(
            "pos: {}, book: {}",
            state.pos(),
            state
                .current_book()
                .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
                .unwrap_or("<unavailable>".to_string())
        );
        let text = Text::with_alignment(
            &text_str,
            display.bounding_box().top_left + Point::new(20, 20),
            text_style,
            Alignment::Left,
        );
        text.bounding_box().draw_styled(&text_bg_style, display)?;
        text.draw(display)?;
    }

    Ok(())
}
