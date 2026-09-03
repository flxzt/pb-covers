use crate::cover;
use anyhow::anyhow;
use core::fmt::Display;
use embedded_graphics_core::geometry::Size;
use image::imageops::FilterType;
use image::{DynamicImage, ImageBuffer, Luma};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub const PB_INKPAD4_SCREEN_SIZE: Size = Size {
    width: 1404,
    height: 1872,
};
pub const PB_TOUCHLUX3_SCREEN_SIZE: Size = Size {
    width: 758,
    height: 1024,
};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub enum Orientation {
    #[default]
    Portrait0Deg,
    Landscape90Deg,
    Portrait180Deg,
    Landscape270Deg,
}

impl Display for Orientation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Orientation::Portrait0Deg => write!(f, "Portrait0Deg"),
            Orientation::Landscape90Deg => write!(f, "Landscape90Deg"),
            Orientation::Portrait180Deg => write!(f, "Portrait180Deg"),
            Orientation::Landscape270Deg => write!(f, "Landscape270Deg"),
        }
    }
}

#[derive(Debug)]
pub struct App {
    // Book search dir. Currently does not recurse.
    #[allow(unused)]
    search_dir: PathBuf,
    // Found books in `search_dir`.
    books: Vec<PathBuf>,
    // Current position in `books` for which a cover should be displayed
    pos: usize,
    /// Whether to show overlay containing state information.
    overlay: bool,
    /// Screen size.
    screen_size: Size,
    /// UI orientation.
    orientation: Orientation,
    // Fetched cover image.
    cover_image: Option<Rc<DynamicImage>>,
    // Buffer for cover image.
    cover_image_buf: Option<Rc<ImageBuffer<Luma<u8>, Vec<u8>>>>,
}

impl App {
    pub fn new(search_dir: PathBuf, screen_size: Size) -> Self {
        let Ok(books) = retrieve_books_in_dir(&search_dir)
            .inspect_err(|err| eprintln!("Unable to read directory: {err:?}"))
        else {
            return Self {
                search_dir,
                books: Vec::new(),
                pos: 0,
                overlay: false,
                screen_size,
                orientation: Orientation::default(),
                cover_image: None,
                cover_image_buf: None,
            };
        };

        Self {
            search_dir,
            books,
            pos: 0,
            overlay: false,
            screen_size,
            orientation: Orientation::default(),
            cover_image: None,
            cover_image_buf: None,
        }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn overlay(&self) -> bool {
        self.overlay
    }

    pub fn toggle_overlay(&mut self) -> bool {
        self.overlay = !self.overlay;
        self.overlay
    }

    pub fn update_screen_size(&mut self, screen_size: Size) {
        self.screen_size = screen_size;
        self.regen_cover_image();
    }

    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    pub fn orientation_cycle_backward(&mut self) -> Orientation {
        self.orientation = match self.orientation {
            Orientation::Portrait0Deg => Orientation::Landscape90Deg,
            Orientation::Landscape90Deg => Orientation::Portrait180Deg,
            Orientation::Portrait180Deg => Orientation::Landscape270Deg,
            Orientation::Landscape270Deg => Orientation::Portrait0Deg,
        };
        self.regen_cover_image_buf();
        self.orientation
    }

    pub fn orientation_cycle_forward(&mut self) -> Orientation {
        self.orientation = match self.orientation {
            Orientation::Portrait0Deg => Orientation::Landscape270Deg,
            Orientation::Landscape270Deg => Orientation::Portrait180Deg,
            Orientation::Portrait180Deg => Orientation::Landscape90Deg,
            Orientation::Landscape90Deg => Orientation::Portrait0Deg,
        };
        self.regen_cover_image_buf();
        self.orientation
    }

    pub fn pos_cycle_backward(&mut self) -> usize {
        if let Some(pos) = self.pos.checked_sub(1) {
            self.pos = pos;
        } else {
            let max_pos = self.books.len().saturating_sub(1);
            self.pos = max_pos;
        }
        self.regen_cover_image();
        self.pos
    }

    pub fn pos_cycle_forward(&mut self) -> usize {
        self.pos = self.pos.saturating_add(1);
        let max_pos = self.books.len().saturating_sub(1);
        if self.pos > max_pos {
            self.pos = 0;
        }
        self.regen_cover_image();
        self.pos
    }

    pub fn current_book(&self) -> Option<&Path> {
        self.books.get(self.pos).map(|p| p.as_ref())
    }

    fn regen_cover_image(&mut self) {
        self.cover_image.take();
        self.cover_image_buf.take();
        let Some(book) = self.current_book() else {
            return;
        };
        let Ok(img) = cover::retrieve_cover(book).inspect_err(|err| {
            eprintln!("Retrieving cover image failed: {err}");
        }) else {
            return;
        };
        let img = match self.orientation {
            Orientation::Portrait0Deg => img,
            Orientation::Landscape90Deg => img.rotate90(),
            Orientation::Portrait180Deg => img.rotate180(),
            Orientation::Landscape270Deg => img.rotate270(),
        };
        let img = Rc::new(img);
        self.cover_image = Some(Rc::clone(&img));
        self.regen_cover_image_buf();
    }

    pub fn regen_cover_image_buf(&mut self) {
        self.cover_image_buf.take();
        let Some(img) = self.cover_image.as_ref() else {
            return;
        };
        let buf = img
            .resize_to_fill(
                self.screen_size.width,
                self.screen_size.height,
                FilterType::Nearest,
            )
            .into_luma8();
        let buf = Rc::new(buf);
        self.cover_image_buf = Some(Rc::clone(&buf));
    }

    #[allow(unused)]
    fn cover_image(&self) -> Option<Rc<DynamicImage>> {
        self.cover_image.as_ref().map(Rc::clone)
    }

    pub fn cover_image_buf(&self) -> Option<Rc<ImageBuffer<Luma<u8>, Vec<u8>>>> {
        self.cover_image_buf.as_ref().map(Rc::clone)
    }
}

fn retrieve_books_in_dir(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Err(anyhow!("Supplied path is not a directory."));
    }
    let books: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|e| {
            let Ok(e) = e else {
                return None;
            };
            let path = e.path();
            if !path.is_file() {
                return None;
            }
            let ext = path.extension()?;
            if ext != "epub" && ext != "pdf" {
                return None;
            };
            Some(path)
        })
        .collect();
    Ok(books)
}
