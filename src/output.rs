use std::fs;
use std::io::stdout;
use std::path::{Path, PathBuf};
use chrono::Local;
use image::DynamicImage;
use log::debug;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::cli::ImageFormat;

#[derive(Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum OutputError {
    #[error("Failed to access system clipboard: {0}")]
    ClipboardFailed(String),

    #[error("Failed to write image to stdout: {0}")]
    StdoutFailed(#[source] image::ImageError),

    #[error("Failed to write bytes to stdout: {0}")]
    StdoutIoFailed(#[source] std::io::Error),

    #[error("Failed to create destination directory '{path}': {source}")]
    CreateDirFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to save image to '{path}': {source}")]
    SaveFailed {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },
}

/// Structured JSON representation of the capture operation result.
#[derive(Debug, Serialize, Deserialize)]
pub struct CaptureResultJson {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    pub clipboard: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}

/// Automatically infer the image format.
/// Prioritizes explicit format if specified; otherwise infers from file extension; defaults to PNG.
pub fn infer_format(path: Option<&Path>, explicit_format: Option<ImageFormat>) -> ImageFormat {
    if let Some(fmt) = explicit_format {
        return fmt;
    }

    if let Some(p) = path {
        if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
            match ext.to_ascii_lowercase().as_str() {
                "jpg" | "jpeg" => return ImageFormat::Jpeg,
                "webp" => return ImageFormat::Webp,
                "png" => return ImageFormat::Png,
                _ => {}
            }
        }
    }

    ImageFormat::Png
}

/// Copy the captured image directly into the system clipboard.
pub fn copy_to_clipboard(image: &DynamicImage) -> Result<(), OutputError> {
    debug!("Converting image to RGBA8 for system clipboard");
    let rgba = image.to_rgba8();
    let width = rgba.width() as usize;
    let height = rgba.height() as usize;
    let raw_bytes = rgba.into_raw();

    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| OutputError::ClipboardFailed(e.to_string()))?;

    let img_data = arboard::ImageData {
        width,
        height,
        bytes: raw_bytes.into(),
    };

    clipboard
        .set_image(img_data)
        .map_err(|e| OutputError::ClipboardFailed(e.to_string()))?;

    debug!("Image successfully written to clipboard buffer");
    Ok(())
}

/// Write encoded image bytes directly to standard output (stdout stream).
pub fn write_to_stdout(image: &DynamicImage, format: ImageFormat) -> Result<(), OutputError> {
    use std::io::{Cursor, Write};

    debug!("Streaming encoded image ({:?}) to stdout", format);
    let mut out = stdout().lock();
    let image_format = match format {
        ImageFormat::Png => image::ImageFormat::Png,
        ImageFormat::Jpeg => image::ImageFormat::Jpeg,
        ImageFormat::Webp => image::ImageFormat::WebP,
    };

    let mut buffer = Cursor::new(Vec::new());
    image
        .write_to(&mut buffer, image_format)
        .map_err(OutputError::StdoutFailed)?;

    out.write_all(buffer.get_ref())
        .map_err(OutputError::StdoutIoFailed)?;

    out.flush()
        .map_err(OutputError::StdoutIoFailed)?;

    Ok(())
}

/// Generates a default timestamped filename, e.g. `screenshot_2026-10-14_00-05-30.png`.
pub fn generate_default_filename(format: ImageFormat) -> String {
    let now = Local::now();
    format!(
        "screenshot_{}.{}",
        now.format("%Y-%m-%d_%H-%M-%S"),
        format.extension()
    )
}

/// Resolves the final destination path for saving an image.
/// If `output` is None, saves to the current directory with a timestamped filename.
/// If `output` is an existing directory, appends the timestamped filename inside it.
/// If `output` is a file path, ensures parent directories exist.
pub fn resolve_output_path(output: Option<&Path>, format: ImageFormat) -> Result<PathBuf, OutputError> {
    let target = match output {
        None => PathBuf::from(generate_default_filename(format)),
        Some(path) => {
            if path.is_dir() {
                path.join(generate_default_filename(format))
            } else {
                path.to_path_buf()
            }
        }
    };

    // Ensure parent directory exists (idempotent)
    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            debug!("Creating parent directory: {:?}", parent);
            fs::create_dir_all(parent).map_err(|e| OutputError::CreateDirFailed {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
    }

    Ok(target)
}

/// Save a `DynamicImage` to the filesystem.
/// Ensures destination directories exist and returns the path to the saved file.
pub fn save_image(
    image: &DynamicImage,
    output: Option<&Path>,
    format: ImageFormat,
) -> Result<PathBuf, OutputError> {
    let file_path = resolve_output_path(output, format)?;
    debug!("Saving image ({:?}) to: {:?}", format, file_path);

    let image_format = match format {
        ImageFormat::Png => image::ImageFormat::Png,
        ImageFormat::Jpeg => image::ImageFormat::Jpeg,
        ImageFormat::Webp => image::ImageFormat::WebP,
    };

    image.save_with_format(&file_path, image_format).map_err(|e| OutputError::SaveFailed {
        path: file_path.clone(),
        source: e,
    })?;

    Ok(file_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn test_infer_format() {
        // Explicit format takes precedence
        assert_eq!(
            infer_format(Some(Path::new("test.jpg")), Some(ImageFormat::Png)),
            ImageFormat::Png
        );

        // Inferred from extension
        assert_eq!(
            infer_format(Some(Path::new("photo.jpg")), None),
            ImageFormat::Jpeg
        );
        assert_eq!(
            infer_format(Some(Path::new("photo.jpeg")), None),
            ImageFormat::Jpeg
        );
        assert_eq!(
            infer_format(Some(Path::new("photo.webp")), None),
            ImageFormat::Webp
        );
        assert_eq!(
            infer_format(Some(Path::new("pic.png")), None),
            ImageFormat::Png
        );

        // Fallback default
        assert_eq!(infer_format(None, None), ImageFormat::Png);
        assert_eq!(infer_format(Some(Path::new("unknown.xyz")), None), ImageFormat::Png);
    }

    #[test]
    fn test_generate_default_filename() {
        let name_png = generate_default_filename(ImageFormat::Png);
        assert!(name_png.starts_with("screenshot_"));
        assert!(name_png.ends_with(".png"));

        let name_jpg = generate_default_filename(ImageFormat::Jpeg);
        assert!(name_jpg.ends_with(".jpg"));

        let name_webp = generate_default_filename(ImageFormat::Webp);
        assert!(name_webp.ends_with(".webp"));
    }

    #[test]
    fn test_resolve_output_path() {
        let path = resolve_output_path(None, ImageFormat::Png).unwrap();
        assert!(path.to_str().unwrap().ends_with(".png"));

        let custom = Path::new("target/test_output_dir/my_pic.webp");
        let resolved = resolve_output_path(Some(custom), ImageFormat::Webp).unwrap();
        assert_eq!(resolved, custom);
        assert!(Path::new("target/test_output_dir").exists());

        // Clean up test dir
        let _ = fs::remove_dir_all("target/test_output_dir");
    }

    #[test]
    fn test_save_image_png_and_webp() {
        // Create 2x2 test image
        let mut img = RgbaImage::new(2, 2);
        img.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        let dynamic = DynamicImage::ImageRgba8(img);

        let test_file_png = Path::new("target/test_scratch/test.png");
        let saved_png = save_image(&dynamic, Some(test_file_png), ImageFormat::Png).unwrap();
        assert_eq!(saved_png, test_file_png);
        assert!(test_file_png.exists());

        let test_file_webp = Path::new("target/test_scratch/test.webp");
        let saved_webp = save_image(&dynamic, Some(test_file_webp), ImageFormat::Webp).unwrap();
        assert_eq!(saved_webp, test_file_webp);
        assert!(test_file_webp.exists());

        // Clean up
        let _ = fs::remove_dir_all("target/test_scratch");
    }

    #[test]
    fn test_clipboard_copy() {
        let mut img = RgbaImage::new(4, 4);
        img.put_pixel(0, 0, Rgba([0, 255, 0, 255]));
        let dynamic = DynamicImage::ImageRgba8(img);

        let result = copy_to_clipboard(&dynamic);
        assert!(result.is_ok(), "Clipboard copy should succeed: {:?}", result.err());
    }
}
