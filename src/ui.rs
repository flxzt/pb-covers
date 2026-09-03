use crate::state::App;
use embedded_graphics::image::ImageRaw;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::pixelcolor::Gray8;
use embedded_graphics::pixelcolor::raw::BigEndian;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Alignment, Text};
use guillotine::style::{AlignItems, JustifyContent};
use guillotine::{CustomElement, ParentElement, Render, StyledElement, StyledFlexContainer};
use image::{ImageBuffer, Luma};
use std::rc::Rc;

#[derive(Debug)]
pub struct CoverImageNode {
    buf: Option<Rc<ImageBuffer<Luma<u8>, Vec<u8>>>>,
}

impl CoverImageNode {
    fn new(buf: Option<Rc<ImageBuffer<Luma<u8>, Vec<u8>>>>) -> Self {
        Self { buf }
    }
}

impl CustomElement<Gray8> for CoverImageNode {
    fn intrinsic_size(&self) -> Size {
        // Always fill as much space as possible
        Size {
            width: u32::MAX,
            height: u32::MAX,
        }
    }

    fn draw<D>(
        &self,
        bounds: &embedded_graphics::primitives::Rectangle,
        theme: &guillotine::Theme<Gray8>,
        target: &mut D,
    ) -> Result<(), D::Error>
    where
        D: embedded_graphics::prelude::DrawTarget<Color = Gray8>,
    {
        if let Some(buf) = self.buf.as_ref() {
            // draw the cover image..
            ImageRaw::<Gray8, BigEndian>::new(buf.as_raw(), buf.width()).draw(target)?;
        } else {
            target.fill_solid(bounds, theme.background)?;
            // ..or a placeholder text
            let text = "- no cover found -";
            Text::with_alignment(
                text,
                bounds.center(),
                MonoTextStyle::new(&FONT_10X20, theme.foreground),
                Alignment::Center,
            )
            .draw(target)?;
        };

        Ok(())
    }
}

impl Render<Gray8, CoverImageNode> for App {
    fn render(
        &self,
        cx: &guillotine::Context<'_, Gray8, CoverImageNode>,
    ) -> impl guillotine::ElementBuilder {
        let text_str;
        let info_text = if self.overlay() {
            text_str = format!(
                "pos: {}, book: {:?}",
                self.pos(),
                self.current_book()
                    .and_then(|p| p.file_name().map(|f| f.to_string_lossy().to_string()))
                    .unwrap_or("<unavailable>".to_string())
            );
            cx.text(&text_str).padding(10).margin(10)
        } else {
            cx.text("")
        };

        cx.column()
            .justify_content(JustifyContent::Start)
            .align_items(AlignItems::Center)
            .child(info_text)
            .child(cx.custom(CoverImageNode::new(self.cover_image_buf())))
    }
}
