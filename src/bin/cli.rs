use anyhow::Context;
use pb_covers::cover::{self};
use std::env;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let book: PathBuf = PathBuf::from(
        env::args()
            .nth(1)
            .expect("Run: cli <book.epub> <cover.png>"),
    );
    let cover: PathBuf = PathBuf::from(
        env::args()
            .nth(2)
            .expect("Run: cli <book.epub> <cover.png>"),
    );
    let cover_image = cover::retrieve_cover(&book).context("Failed to retrieve book cover")?;
    cover_image.save(cover)?;
    Ok(())
}
