use chrono::Local;
use image::DynamicImage;
use log::debug;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use thiserror::Error;

use crate::cli::ImageFormat;

/// Atomic sequence counter for collision-free auto-generated filenames within the process.
static SEQUENCE_COUNTER: AtomicU32 = AtomicU32::new(0);

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

    #[error("Explicit format '{explicit:?}' conflicts with file extension of '{path}' (which indicates '{inferred:?}')")]
    FormatMismatch {
        explicit: ImageFormat,
        inferred: ImageFormat,
        path: PathBuf,
    },

    #[error("Invalid destination path: {0}")]
    InvalidPath(String),
}

/// Standardized error codes for machine-readable JSON responses and process exit codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    BadArgs,
    NoMonitor,
    MonitorNotFound,
    RegionOutOfBounds,
    PermissionDenied,
    CaptureFailed,
    ClipboardFailed,
    EncodeFailed,
    SaveFailed,
    FormatMismatch,
    IoError,
    Unknown,
}

impl ErrorCode {
    pub fn exit_code(&self) -> i32 {
        match self {
            ErrorCode::BadArgs => 1,
            ErrorCode::NoMonitor => 2,
            ErrorCode::MonitorNotFound => 3,
            ErrorCode::RegionOutOfBounds => 4,
            ErrorCode::PermissionDenied => 5,
            ErrorCode::CaptureFailed => 6,
            ErrorCode::ClipboardFailed => 7,
            ErrorCode::EncodeFailed => 8,
            ErrorCode::SaveFailed => 9,
            ErrorCode::FormatMismatch => 10,
            ErrorCode::IoError => 11,
            ErrorCode::Unknown => 127,
        }
    }
}

/// Detailed machine-readable error payload complying with the public schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub error_code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    pub exit_code: i32,
}

impl ApiError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        let exit_code = code.exit_code();
        Self {
            error_code: code,
            message: message.into(),
            stage: None,
            suggestion: None,
            exit_code,
        }
    }

    pub fn with_stage(mut self, stage: impl Into<String>) -> Self {
        self.stage = Some(stage.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

impl From<OutputError> for ApiError {
    fn from(err: OutputError) -> Self {
        match err {
            OutputError::ClipboardFailed(msg) => ApiError::new(ErrorCode::ClipboardFailed, msg)
                .with_stage("clipboard")
                .with_suggestion("Ensure clipboard is accessible and not held exclusively by another process."),
            OutputError::StdoutFailed(e) => ApiError::new(ErrorCode::EncodeFailed, e.to_string())
                .with_stage("stdout_encode")
                .with_suggestion("Ensure the chosen format supports the image dimensions."),
            OutputError::StdoutIoFailed(e) => ApiError::new(ErrorCode::IoError, e.to_string())
                .with_stage("stdout_stream")
                .with_suggestion("Check if downstream pipe consumer is accepting data."),
            OutputError::CreateDirFailed { path, source } => ApiError::new(
                ErrorCode::SaveFailed,
                format!("Failed to create destination directory '{}': {}", path.display(), source),
            )
            .with_stage("directory_creation")
            .with_suggestion("Verify parent directory permissions and disk space."),
            OutputError::SaveFailed { path, source } => ApiError::new(
                ErrorCode::SaveFailed,
                format!("Failed to save image to '{}': {}", path.display(), source),
            )
            .with_stage("save_file")
            .with_suggestion("Verify write permissions and disk space."),
            OutputError::FormatMismatch { explicit, inferred, path } => ApiError::new(
                ErrorCode::FormatMismatch,
                format!(
                    "Explicit format '{:?}' conflicts with file extension of '{}' (which implies '{:?}')",
                    explicit, path.display(), inferred
                ),
            )
            .with_stage("argument_validation")
            .with_suggestion("Specify matching file extension or omit explicit --format."),
            OutputError::InvalidPath(msg) => ApiError::new(ErrorCode::BadArgs, msg)
                .with_stage("argument_validation")
                .with_suggestion("Provide a valid file or directory destination."),
        }
    }
}

/// Unified API response wrapper for JSON output mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse<T> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn failure(error: ApiError) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(error),
        }
    }
}

/// Structured JSON representation of the capture operation result.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResultJson {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    pub clipboard: bool,
    #[serde(skip_serializing_if = "Option::is_none", alias = "size_bytes")]
    pub size_bytes: Option<u64>,
}

/// Destination target for screenshot output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputDestination {
    Stdout,
    Auto,
    Directory(PathBuf),
    File(PathBuf),
}

/// Determines whether a path is intended as a directory destination.
pub fn is_directory_target(path: &Path) -> bool {
    if path.is_dir() {
        return true;
    }

    let path_str = path.to_string_lossy();
    if path_str.ends_with('/') || path_str.ends_with('\\') {
        return true;
    }

    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if name == "." || name == ".." {
            return true;
        }
    }

    false
}

/// Resolves user CLI arguments into an unambiguous `OutputDestination`.
pub fn determine_output_destination(
    output: Option<&Path>,
    output_file: Option<&Path>,
    output_dir: Option<&Path>,
) -> Result<OutputDestination, OutputError> {
    if let Some(dir) = output_dir {
        if dir.is_file() {
            return Err(OutputError::InvalidPath(format!(
                "Path '{}' is an existing file, but was specified as --output-dir",
                dir.display()
            )));
        }
        return Ok(OutputDestination::Directory(dir.to_path_buf()));
    }

    if let Some(file) = output_file {
        if is_directory_target(file) {
            return Err(OutputError::InvalidPath(format!(
                "Path '{}' is a directory, but was specified as --output-file",
                file.display()
            )));
        }
        return Ok(OutputDestination::File(file.to_path_buf()));
    }

    if let Some(path) = output {
        if path.as_os_str() == "-" {
            return Ok(OutputDestination::Stdout);
        }
        if is_directory_target(path) {
            return Ok(OutputDestination::Directory(path.to_path_buf()));
        }
        return Ok(OutputDestination::File(path.to_path_buf()));
    }

    Ok(OutputDestination::Auto)
}

/// Validates explicit format against destination file extension and infers image format.
pub fn validate_and_infer_format(
    destination: &OutputDestination,
    explicit_format: Option<ImageFormat>,
) -> Result<ImageFormat, OutputError> {
    if let OutputDestination::File(path) = destination {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let inferred = match ext.to_ascii_lowercase().as_str() {
                "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
                "webp" => Some(ImageFormat::Webp),
                "png" => Some(ImageFormat::Png),
                _ => None,
            };

            if let Some(inf) = inferred {
                if let Some(exp) = explicit_format {
                    if exp != inf {
                        return Err(OutputError::FormatMismatch {
                            explicit: exp,
                            inferred: inf,
                            path: path.clone(),
                        });
                    }
                }
                return Ok(inf);
            }
        }
    }

    Ok(explicit_format.unwrap_or(ImageFormat::Png))
}

/// Automatically infer the image format (fallback for backwards compatibility).
#[allow(dead_code)]
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

    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| OutputError::ClipboardFailed(e.to_string()))?;

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

    let initial_capacity = (image.width() as usize * image.height() as usize).min(1024 * 1024);
    let mut buffer = Cursor::new(Vec::with_capacity(initial_capacity));
    image
        .write_to(&mut buffer, image_format)
        .map_err(OutputError::StdoutFailed)?;

    if let Err(e) = out.write_all(buffer.get_ref()) {
        if e.kind() == std::io::ErrorKind::BrokenPipe {
            debug!("Downstream pipe closed by consumer (BrokenPipe)");
            return Ok(());
        }
        return Err(OutputError::StdoutIoFailed(e));
    }

    if let Err(e) = out.flush() {
        if e.kind() == std::io::ErrorKind::BrokenPipe {
            debug!("Downstream pipe closed by consumer during flush (BrokenPipe)");
            return Ok(());
        }
        return Err(OutputError::StdoutIoFailed(e));
    }

    Ok(())
}

/// Generates a default collision-resistant timestamped filename, e.g. `screenshot_2026-10-15_23-55-12.123_0001.png`.
pub fn generate_default_filename(format: ImageFormat) -> String {
    let now = Local::now();
    let seq = SEQUENCE_COUNTER.fetch_add(1, Ordering::Relaxed) % 10000;
    format!(
        "screenshot_{}_{:04}.{}",
        now.format("%Y-%m-%d_%H-%M-%S%.3f"),
        seq,
        format.extension()
    )
}

/// Resolves a non-colliding path inside the given directory for auto-generated files.
pub fn resolve_unique_auto_path(dir: &Path, format: ImageFormat) -> Result<PathBuf, OutputError> {
    for _ in 0..100 {
        let candidate = dir.join(generate_default_filename(format));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    // High-entropy fallback if 100 fast attempts collided
    let now = Local::now();
    let nanos = now.timestamp_subsec_nanos();
    let candidate = dir.join(format!(
        "screenshot_{}_{}_{:09}.{}",
        now.format("%Y-%m-%d_%H-%M-%S"),
        std::process::id(),
        nanos,
        format.extension()
    ));
    Ok(candidate)
}

/// Resolves the final destination path for saving an image.
pub fn resolve_output_path(
    destination: &OutputDestination,
    format: ImageFormat,
) -> Result<PathBuf, OutputError> {
    match destination {
        OutputDestination::Stdout => Err(OutputError::InvalidPath(
            "Cannot resolve filesystem path for stdout destination".to_string(),
        )),
        OutputDestination::Auto => {
            let cwd = Path::new(".");
            resolve_unique_auto_path(cwd, format)
        }
        OutputDestination::Directory(dir) => {
            if !dir.exists() {
                debug!("Creating destination directory: {:?}", dir);
                fs::create_dir_all(dir).map_err(|e| OutputError::CreateDirFailed {
                    path: dir.to_path_buf(),
                    source: e,
                })?;
            }
            resolve_unique_auto_path(dir, format)
        }
        OutputDestination::File(file) => {
            if let Some(parent) = file.parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    debug!("Creating parent directory: {:?}", parent);
                    fs::create_dir_all(parent).map_err(|e| OutputError::CreateDirFailed {
                        path: parent.to_path_buf(),
                        source: e,
                    })?;
                }
            }
            // User-specified file paths preserve intentional overwrite semantics
            Ok(file.clone())
        }
    }
}

/// Save a `DynamicImage` to the filesystem destination.
pub fn save_image(
    image: &DynamicImage,
    destination: &OutputDestination,
    format: ImageFormat,
) -> Result<PathBuf, OutputError> {
    let file_path = resolve_output_path(destination, format)?;
    debug!("Saving image ({:?}) to: {:?}", format, file_path);

    let image_format = match format {
        ImageFormat::Png => image::ImageFormat::Png,
        ImageFormat::Jpeg => image::ImageFormat::Jpeg,
        ImageFormat::Webp => image::ImageFormat::WebP,
    };

    image
        .save_with_format(&file_path, image_format)
        .map_err(|e| OutputError::SaveFailed {
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
        assert_eq!(
            infer_format(Some(Path::new("test.jpg")), Some(ImageFormat::Png)),
            ImageFormat::Png
        );
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
        assert_eq!(infer_format(None, None), ImageFormat::Png);
        assert_eq!(
            infer_format(Some(Path::new("unknown.xyz")), None),
            ImageFormat::Png
        );
    }

    #[test]
    fn test_generate_default_filename_uniqueness() {
        let name1 = generate_default_filename(ImageFormat::Png);
        let name2 = generate_default_filename(ImageFormat::Png);
        assert_ne!(
            name1, name2,
            "Consecutive generated filenames must be unique"
        );
        assert!(name1.starts_with("screenshot_"));
        assert!(name1.ends_with(".png"));
    }

    #[test]
    fn test_is_directory_target() {
        assert!(is_directory_target(Path::new("foo/bar/")));
        assert!(is_directory_target(Path::new("foo\\bar\\")));
        assert!(is_directory_target(Path::new(".")));
        assert!(is_directory_target(Path::new("..")));
        assert!(!is_directory_target(Path::new("foo/bar/pic.png")));
    }

    #[test]
    fn test_determine_output_destination() {
        assert_eq!(
            determine_output_destination(None, None, None).unwrap(),
            OutputDestination::Auto
        );
        assert_eq!(
            determine_output_destination(Some(Path::new("-")), None, None).unwrap(),
            OutputDestination::Stdout
        );
        assert_eq!(
            determine_output_destination(Some(Path::new("dir/")), None, None).unwrap(),
            OutputDestination::Directory(PathBuf::from("dir/"))
        );
        assert_eq!(
            determine_output_destination(Some(Path::new("out.png")), None, None).unwrap(),
            OutputDestination::File(PathBuf::from("out.png"))
        );
        assert_eq!(
            determine_output_destination(None, Some(Path::new("custom.png")), None).unwrap(),
            OutputDestination::File(PathBuf::from("custom.png"))
        );
        assert_eq!(
            determine_output_destination(None, None, Some(Path::new("screenshots"))).unwrap(),
            OutputDestination::Directory(PathBuf::from("screenshots"))
        );
    }

    #[test]
    fn test_validate_and_infer_format_mismatch() {
        let dest = OutputDestination::File(PathBuf::from("shot.jpg"));
        let err = validate_and_infer_format(&dest, Some(ImageFormat::Png)).unwrap_err();
        match err {
            OutputError::FormatMismatch {
                explicit, inferred, ..
            } => {
                assert_eq!(explicit, ImageFormat::Png);
                assert_eq!(inferred, ImageFormat::Jpeg);
            }
            _ => panic!("Expected FormatMismatch error"),
        }

        // Matching format succeeds
        let ok_fmt = validate_and_infer_format(&dest, Some(ImageFormat::Jpeg)).unwrap();
        assert_eq!(ok_fmt, ImageFormat::Jpeg);

        // Inferred when omitted
        let inf_fmt = validate_and_infer_format(&dest, None).unwrap();
        assert_eq!(inf_fmt, ImageFormat::Jpeg);
    }

    #[test]
    fn test_resolve_output_path() {
        let dest = OutputDestination::Auto;
        let path = resolve_output_path(&dest, ImageFormat::Png).unwrap();
        assert!(path.to_str().unwrap().ends_with(".png"));

        let custom = OutputDestination::File(PathBuf::from("target/test_output_dir/my_pic.webp"));
        let resolved = resolve_output_path(&custom, ImageFormat::Webp).unwrap();
        assert_eq!(
            resolved,
            PathBuf::from("target/test_output_dir/my_pic.webp")
        );
        assert!(Path::new("target/test_output_dir").exists());

        // Clean up test dir
        let _ = fs::remove_dir_all("target/test_output_dir");
    }

    #[test]
    fn test_save_image_png_and_webp() {
        let mut img = RgbaImage::new(2, 2);
        img.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        let dynamic = DynamicImage::ImageRgba8(img);

        let test_file_png = OutputDestination::File(PathBuf::from("target/test_scratch/test.png"));
        let saved_png = save_image(&dynamic, &test_file_png, ImageFormat::Png).unwrap();
        assert_eq!(saved_png, PathBuf::from("target/test_scratch/test.png"));
        assert!(saved_png.exists());

        let test_file_webp =
            OutputDestination::File(PathBuf::from("target/test_scratch/test.webp"));
        let saved_webp = save_image(&dynamic, &test_file_webp, ImageFormat::Webp).unwrap();
        assert_eq!(saved_webp, PathBuf::from("target/test_scratch/test.webp"));
        assert!(saved_webp.exists());

        // Clean up
        let _ = fs::remove_dir_all("target/test_scratch");
    }

    #[test]
    #[ignore = "requires system clipboard access"]
    fn test_clipboard_copy() {
        let mut img = RgbaImage::new(4, 4);
        img.put_pixel(0, 0, Rgba([0, 255, 0, 255]));
        let dynamic = DynamicImage::ImageRgba8(img);

        let result = copy_to_clipboard(&dynamic);
        assert!(
            result.is_ok(),
            "Clipboard copy should succeed: {:?}",
            result.err()
        );
    }
}
