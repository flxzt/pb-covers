use anyhow::{Context, anyhow};
use hayro::hayro_interpret;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::AlphaColor;
use image::{DynamicImage, ImageReader, RgbaImage};
use std::fs::{self, File};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

pub fn retrieve_cover(book: &Path) -> anyhow::Result<DynamicImage> {
    match book.extension().context("Book has no file extension")? {
        ext if ext == "epub" => retrieve_epub_cover(book).context("Unable to retrieve epub cover"),
        ext if ext == "pdf" => retrieve_pdf_cover(book).context("Unable to retrieve PDF cover"),
        ext => Err(anyhow!("Book has unsupported file extension '{ext:?}'")),
    }
}

fn retrieve_epub_cover(epub_file: &Path) -> anyhow::Result<DynamicImage> {
    eprintln!("Retrieving cover from epub '{}'", epub_file.display());
    let fh = File::open(epub_file)?;
    let mut str_buf = String::new();
    let mut data_buf: Vec<u8> = Vec::new();
    let mut zip = zip::ZipArchive::new(fh)?;

    let container = {
        let idx = zip
            .index_for_path(PathBuf::new().join("META-INF").join("container.xml"))
            .context("Failed to get container XML index")?;
        let mut file = zip
            .by_index(idx)
            .context("Failed to get container XML file.")?;
        file.read_to_string(&mut str_buf)
            .context("Failed to read container XML data")?;
        roxmltree::Document::parse(&str_buf).context("Container XML not valid XML")?
    };

    let rootfile_path = container
        .root_element()
        .children()
        .find(|n| n.has_tag_name("rootfiles"))
        .and_then(|n| {
            n.children()
                .find(|n| n.has_tag_name("rootfile") && n.has_attribute("full-path"))
        })
        .and_then(|n| n.attribute("full-path").map(PathBuf::from))
        .context("Container XML does not contain rootfile child with attribute 'full-path'")?;
    let rootfile = {
        let idx = zip
            .index_for_path(&rootfile_path)
            .context("Failed to get rootfile index")?;
        let mut file = zip.by_index(idx).context("Failed to get rootfile.")?;
        str_buf.clear();
        file.read_to_string(&mut str_buf)
            .context("Failed to read rootfile data")?;
        roxmltree::Document::parse(&str_buf).context("rootfile not valid XML")?
    };

    let cover_path = rootfile
        .root_element()
        .descendants()
        .find_map(|n| {
            // <metadata ..>
            //     ...
            //     <meta content="my-cover-image" name="cover"/>
            //     ...
            // </metadata>
            if n.attribute("name") != Some("cover") {
                return None;
            };
            let id = n.attribute("content")?;
            // <manifest>
            // ...
            // <item id="my-cover-image" href="images/cover.png" ... />
            // ...
            // </manifest>
            rootfile
                .root_element()
                .descendants()
                .find(|n| n.attribute("id") == Some(id))
        })
        .and_then(|n| n.attribute("href").map(PathBuf::from))
        .context("Unable to find cover path in rootfile")?;
    let cover_full_path = rootfile_path
        .parent()
        .context("Failed to get rootfile parent dir")?
        .join(cover_path);
    let cover = {
        let idx = zip
            .index_for_path(cover_full_path)
            .context("Failed to get cover index")?;
        let mut file = zip.by_index(idx).context("Failed to get cover.")?;
        file.read_to_end(&mut data_buf)
            .context("Failed to read cover data")?;
        ImageReader::new(Cursor::new(&data_buf))
            .with_guessed_format()?
            .decode()?
    };

    Ok(cover)
}

fn retrieve_pdf_cover(pdf_file: &Path) -> anyhow::Result<DynamicImage> {
    eprintln!("Retrieving cover from PDF '{}'", pdf_file.display());
    let data = fs::read(pdf_file).context("Failed to read PDF file")?;
    let pdf =
        Pdf::new(data).map_err(|err| anyhow!("Failed to create PDF object from data: {err:?}"))?;
    let page = pdf.pages().first().context("PDF has no first page")?;
    let pixmap = hayro::render(
        page,
        &hayro::RenderCache::new(),
        &hayro_interpret::InterpreterSettings::default(),
        &hayro::RenderSettings {
            bg_color: AlphaColor::WHITE,
            ..Default::default()
        },
    );
    let image = DynamicImage::ImageRgba8(
        RgbaImage::from_raw(
            pixmap.width() as u32,
            pixmap.height() as u32,
            pixmap.data_as_u8_slice().to_vec(),
        )
        .context("Unable to create image from PDF page")?,
    );
    Ok(image)
}
