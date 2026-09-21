use crate::cover;
use anyhow::anyhow;
use embedded_graphics::prelude::Size;
use image::imageops::FilterType;
use image::{DynamicImage, GrayImage};
use inkview::screen::ScreenOrientation;
use std::fs;
use std::path::{Path, PathBuf};

pub struct State {
    // Book search dir. Currently does not recurse.
    #[allow(unused)]
    search_dir: PathBuf,
    // Found books in `search_dir`.
    books: Vec<PathBuf>,
    // Current position in `books` for which a cover should be displayed
    pos: usize,
    /// Whether to show overlay containing state information.
    overlay: bool,
    /// The orientatino as interpreteted by inkview. Valid values: 0-3.
    orientation: ScreenOrientation,
    // image and buffer are invalidated when cycling through `pos` and lazily recomputed on demand.
    cover_image: Option<DynamicImage>,
    cover_image_buf: Option<GrayImage>,
}

impl State {
    pub fn new(search_dir: PathBuf) -> Self {
        let Ok(books) = retrieve_books_in_dir(&search_dir)
            .inspect_err(|err| eprintln!("Unable to read directory: {err:?}"))
        else {
            return Self {
                search_dir,
                books: Vec::new(),
                pos: 0,
                overlay: false,
                orientation: ScreenOrientation::default(),
                cover_image: None,
                cover_image_buf: None,
            };
        };

        Self {
            search_dir,
            books,
            pos: 0,
            overlay: false,
            orientation: ScreenOrientation::default(),
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

    pub fn orientation(&self) -> ScreenOrientation {
        self.orientation
    }

    pub fn orientation_cycle_backward(&mut self) -> ScreenOrientation {
        self.orientation = match self.orientation {
            ScreenOrientation::Portrait0Deg => ScreenOrientation::Landscape90Deg,
            ScreenOrientation::Landscape90Deg => ScreenOrientation::Portrait180Deg,
            ScreenOrientation::Portrait180Deg => ScreenOrientation::Landscape270Deg,
            ScreenOrientation::Landscape270Deg => ScreenOrientation::Portrait0Deg,
        };
        self.cover_image_buf.take();
        self.orientation
    }

    pub fn orientation_cycle_forward(&mut self) -> ScreenOrientation {
        self.orientation = match self.orientation {
            ScreenOrientation::Portrait0Deg => ScreenOrientation::Landscape270Deg,
            ScreenOrientation::Landscape270Deg => ScreenOrientation::Portrait180Deg,
            ScreenOrientation::Portrait180Deg => ScreenOrientation::Landscape90Deg,
            ScreenOrientation::Landscape90Deg => ScreenOrientation::Portrait0Deg,
        };
        self.cover_image_buf.take();
        self.orientation
    }

    pub fn pos_cycle_backward(&mut self) -> usize {
        self.pos = self.pos.saturating_sub(1);
        let max_pos = self.books.len().saturating_sub(1);
        if self.pos == 0 {
            self.pos = max_pos;
        }
        self.cover_image.take();
        self.cover_image_buf.take();
        self.pos
    }

    pub fn pos_cycle_forward(&mut self) -> usize {
        self.pos = self.pos.saturating_add(1);
        let max_pos = self.books.len().saturating_sub(1);
        if self.pos > max_pos {
            self.pos = 0;
        }
        self.cover_image.take();
        self.cover_image_buf.take();
        self.pos
    }

    pub fn current_book(&self) -> Option<&Path> {
        self.books.get(self.pos).map(|p| p.as_ref())
    }

    pub fn cover_image(&mut self) -> Option<&DynamicImage> {
        if let Some(ref cover_image) = self.cover_image {
            return Some(cover_image);
        };
        let book = self.current_book()?;
        let cover_image = cover::retrieve_cover(book)
            .inspect_err(|err| eprintln!("Failed to retrieve book cover: {err:?}"))
            .ok()?;
        self.cover_image = Some(cover_image);
        self.cover_image.as_ref()
    }

    pub fn cover_image_buf(&mut self, size: Size) -> Option<&GrayImage> {
        if let Some(ref buf) = self.cover_image_buf {
            return Some(buf);
        }
        let orientation = self.orientation;
        let cover_image = self.cover_image()?;
        let cover_image = match orientation {
            ScreenOrientation::Portrait0Deg => cover_image,
            ScreenOrientation::Landscape90Deg => &cover_image.rotate90(),
            ScreenOrientation::Portrait180Deg => &cover_image.rotate180(),
            ScreenOrientation::Landscape270Deg => &cover_image.rotate270(),
        };
        let buf = cover_image
            .resize_to_fill(size.width, size.height, FilterType::Nearest)
            .to_luma8();
        self.cover_image_buf = Some(buf);
        self.cover_image_buf.as_ref()
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
