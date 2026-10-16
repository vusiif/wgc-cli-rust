use image::{DynamicImage, RgbaImage};
use log::debug;
use thiserror::Error;
use xcap::Monitor;

use crate::cli::Region;

#[derive(Error, Debug)]
pub enum CaptureError {
    #[error("No monitors found on system")]
    NoMonitorsFound,

    #[error("Monitor index {0} is out of bounds (found {1} monitors)")]
    MonitorNotFound(usize, usize),

    #[error("Failed to list monitors: {0}")]
    ListMonitorsFailed(#[source] xcap::XCapError),

    #[error("Failed to query monitor properties: {0}")]
    MonitorPropertyFailed(#[source] xcap::XCapError),

    #[error("Failed to capture monitor: {0}")]
    CaptureFailed(#[source] xcap::XCapError),

    #[error("Invalid scale factor {0}: scale factor must be a finite, positive number")]
    InvalidScaleFactor(f32),

    #[error(
        "Specified crop region (x={x}, y={y}, w={width}, h={height}) is out of monitor bounds ({monitor_w}x{monitor_h})"
    )]
    RegionOutOfBounds {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        monitor_w: u32,
        monitor_h: u32,
    },
}

impl From<CaptureError> for crate::output::ApiError {
    fn from(err: CaptureError) -> Self {
        use crate::output::{ApiError, ErrorCode};
        match err {
            CaptureError::NoMonitorsFound => {
                ApiError::new(ErrorCode::NoMonitor, "No monitors found on system.")
                    .with_stage("monitor_selection")
                    .with_suggestion("Ensure at least one active display is connected and powered on.")
            }
            CaptureError::MonitorNotFound(req, avail) => ApiError::new(
                ErrorCode::MonitorNotFound,
                format!("Monitor index {req} is out of bounds (found {avail} monitors)."),
            )
            .with_stage("monitor_selection")
            .with_suggestion("Run 'rgc-cli --list-monitors' to see all valid monitor indices."),
            CaptureError::ListMonitorsFailed(e) => ApiError::new(
                ErrorCode::CaptureFailed,
                format!("Failed to list monitors: {e}"),
            )
            .with_stage("monitor_enumeration")
            .with_suggestion("Verify display server permissions or graphics drivers."),
            CaptureError::MonitorPropertyFailed(e) => ApiError::new(
                ErrorCode::CaptureFailed,
                format!("Failed to query monitor properties: {e}"),
            )
            .with_stage("monitor_properties")
            .with_suggestion("Verify display driver stability."),
            CaptureError::CaptureFailed(e) => ApiError::new(
                ErrorCode::CaptureFailed,
                format!("Failed to capture monitor: {e}"),
            )
            .with_stage("screen_capture")
            .with_suggestion("Check display capture permissions in OS privacy settings."),
            CaptureError::InvalidScaleFactor(factor) => ApiError::new(
                ErrorCode::CaptureFailed,
                format!("Invalid scale factor {factor}: must be a finite, positive number."),
            )
            .with_stage("dpi_scaling")
            .with_suggestion("Check OS display scaling settings."),
            CaptureError::RegionOutOfBounds {
                x,
                y,
                width,
                height,
                monitor_w,
                monitor_h,
            } => ApiError::new(
                ErrorCode::RegionOutOfBounds,
                format!(
                    "Specified crop region (x={x}, y={y}, w={width}, h={height}) is out of monitor bounds ({monitor_w}x{monitor_h})."
                ),
            )
            .with_stage("region_validation")
            .with_suggestion("Adjust region coordinates to fit within the selected monitor boundaries."),
        }
    }
}

/// Information about a connected display/monitor.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MonitorInfo {
    pub index: usize,
    pub name: String,
    pub is_primary: bool,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
}

/// Retrieve all connected monitors from the system.
pub fn get_monitors() -> Result<Vec<Monitor>, CaptureError> {
    Monitor::all().map_err(CaptureError::ListMonitorsFailed)
}

/// Retrieve friendly display information for all connected monitors.
pub fn get_monitor_infos() -> Result<Vec<MonitorInfo>, CaptureError> {
    let monitors = get_monitors()?;
    let mut infos = Vec::with_capacity(monitors.len());

    for (index, monitor) in monitors.iter().enumerate() {
        let name = monitor.name().unwrap_or_else(|_| format!("Monitor #{index}"));
        let is_primary = monitor.is_primary().unwrap_or(false);
        let width = monitor.width().map_err(CaptureError::MonitorPropertyFailed)?;
        let height = monitor.height().map_err(CaptureError::MonitorPropertyFailed)?;
        let scale_factor = monitor.scale_factor().unwrap_or(1.0);

        infos.push(MonitorInfo {
            index,
            name,
            is_primary,
            width,
            height,
            scale_factor,
        });
    }

    Ok(infos)
}

/// Find the primary monitor index, falling back to index 0 if not explicitly designated.
pub fn find_primary_monitor_index(monitors: &[Monitor]) -> usize {
    for (idx, m) in monitors.iter().enumerate() {
        if m.is_primary().unwrap_or(false) {
            return idx;
        }
    }
    0
}

/// Map user logical coordinates to physical pixels taking into account the monitor's DPI scale factor.
/// Employs overflow-safe arithmetic and validates scaling factors and bounds.
pub fn logical_to_physical_region(
    region: &Region,
    scale_factor: f32,
    monitor_width: u32,
    monitor_height: u32,
) -> Result<(u32, u32, u32, u32), CaptureError> {
    if !scale_factor.is_finite() || scale_factor <= 0.0 {
        return Err(CaptureError::InvalidScaleFactor(scale_factor));
    }

    if region.x < 0 || region.y < 0 {
        return Err(CaptureError::RegionOutOfBounds {
            x: region.x.max(0) as u32,
            y: region.y.max(0) as u32,
            width: region.width,
            height: region.height,
            monitor_w: monitor_width,
            monitor_h: monitor_height,
        });
    }

    let scale = scale_factor as f64;
    let phys_x_f = (region.x as f64 * scale).round();
    let phys_y_f = (region.y as f64 * scale).round();
    let phys_w_f = (region.width as f64 * scale).round();
    let phys_h_f = (region.height as f64 * scale).round();

    if phys_x_f < 0.0
        || phys_y_f < 0.0
        || phys_w_f <= 0.0
        || phys_h_f <= 0.0
        || phys_x_f > u32::MAX as f64
        || phys_y_f > u32::MAX as f64
        || phys_w_f > u32::MAX as f64
        || phys_h_f > u32::MAX as f64
    {
        return Err(CaptureError::RegionOutOfBounds {
            x: phys_x_f.clamp(0.0, u32::MAX as f64) as u32,
            y: phys_y_f.clamp(0.0, u32::MAX as f64) as u32,
            width: phys_w_f.clamp(0.0, u32::MAX as f64) as u32,
            height: phys_h_f.clamp(0.0, u32::MAX as f64) as u32,
            monitor_w: monitor_width,
            monitor_h: monitor_height,
        });
    }

    let phys_x = phys_x_f as u32;
    let phys_y = phys_y_f as u32;
    let phys_w = phys_w_f as u32;
    let phys_h = phys_h_f as u32;

    debug!(
        "Translated region: logical({:?}) * scale({}) -> physical(x={}, y={}, w={}, h={})",
        region, scale_factor, phys_x, phys_y, phys_w, phys_h
    );

    // Overflow-safe bounds check:
    // 1. Starting coordinate must be within monitor dimensions.
    // 2. Width/Height must not exceed the remaining room from the start coordinate.
    // 3. Width/Height must be greater than zero.
    let is_valid = phys_x < monitor_width
        && phys_y < monitor_height
        && phys_w <= monitor_width - phys_x
        && phys_h <= monitor_height - phys_y
        && phys_w > 0
        && phys_h > 0;

    if !is_valid {
        return Err(CaptureError::RegionOutOfBounds {
            x: phys_x,
            y: phys_y,
            width: phys_w,
            height: phys_h,
            monitor_w: monitor_width,
            monitor_h: monitor_height,
        });
    }

    Ok((phys_x, phys_y, phys_w, phys_h))
}

/// Capture the primary monitor and return raw RGBA image data in memory.
#[allow(dead_code)]
pub fn capture_primary_monitor() -> Result<RgbaImage, CaptureError> {
    let monitors = get_monitors()?;
    if monitors.is_empty() {
        return Err(CaptureError::NoMonitorsFound);
    }

    let primary_idx = find_primary_monitor_index(&monitors);
    debug!("Capturing primary monitor at index {}", primary_idx);

    monitors[primary_idx]
        .capture_image()
        .map_err(CaptureError::CaptureFailed)
}

/// Capture a screenshot from a specific monitor index (or primary by default),
/// optionally cropping to a specified logical region.
pub fn capture_screen(
    monitor_index: Option<usize>,
    region: Option<&Region>,
) -> Result<DynamicImage, CaptureError> {
    let monitors = get_monitors()?;
    if monitors.is_empty() {
        return Err(CaptureError::NoMonitorsFound);
    }

    let target_idx = match monitor_index {
        Some(idx) => {
            if idx >= monitors.len() {
                return Err(CaptureError::MonitorNotFound(idx, monitors.len()));
            }
            idx
        }
        None => find_primary_monitor_index(&monitors),
    };

    let target_monitor = &monitors[target_idx];
    debug!(
        "Selected monitor #{}: name={:?}, primary={:?}",
        target_idx,
        target_monitor.name(),
        target_monitor.is_primary()
    );

    let raw_image = target_monitor
        .capture_image()
        .map_err(CaptureError::CaptureFailed)?;

    if let Some(reg) = region {
        let width = target_monitor
            .width()
            .map_err(CaptureError::MonitorPropertyFailed)?;
        let height = target_monitor
            .height()
            .map_err(CaptureError::MonitorPropertyFailed)?;
        let scale_factor = target_monitor.scale_factor().unwrap_or(1.0);

        let (x, y, w, h) = logical_to_physical_region(reg, scale_factor, width, height)?;
        debug!("Cropping image to physical region: x={}, y={}, w={}, h={}", x, y, w, h);

        // Zero-cost sub-image cropping view converted to owned buffer
        let cropped = image::imageops::crop_imm(&raw_image, x, y, w, h).to_image();
        Ok(DynamicImage::ImageRgba8(cropped))
    } else {
        Ok(DynamicImage::ImageRgba8(raw_image))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires graphical desktop session / physical monitor"]
    fn test_list_and_capture_primary() {
        let monitors = get_monitors().expect("Should query monitors");
        assert!(!monitors.is_empty());

        let infos = get_monitor_infos().expect("Should query monitor info");
        assert_eq!(infos.len(), monitors.len());

        let primary_img = capture_primary_monitor().expect("Capture primary monitor should succeed");
        assert!(primary_img.width() > 0);
        assert!(primary_img.height() > 0);
    }

    #[test]
    #[ignore = "requires graphical desktop session / physical monitor"]
    fn test_capture_screen_with_region() {
        let region = Region {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };

        let captured = capture_screen(Some(0), Some(&region)).expect("Capture with region should succeed");
        assert!(captured.width() >= 100);
        assert!(captured.height() >= 100);
    }

    #[test]
    fn test_capture_screen_invalid_monitor_index() {
        let err = capture_screen(Some(9999), None).unwrap_err();
        match err {
            CaptureError::MonitorNotFound(idx, _) => assert_eq!(idx, 9999),
            _ => panic!("Expected MonitorNotFound error"),
        }
    }

    #[test]
    fn test_logical_to_physical_region() {
        let region = Region {
            x: 100,
            y: 100,
            width: 200,
            height: 200,
        };

        // Scale 1.5x on 1920x1080 display
        let (x, y, w, h) = logical_to_physical_region(&region, 1.5, 1920, 1080).unwrap();
        assert_eq!(x, 150);
        assert_eq!(y, 150);
        assert_eq!(w, 300);
        assert_eq!(h, 300);

        // Edge-aligned boundary: exactly fits monitor (100 + 1820 = 1920, 100 + 980 = 1080)
        let edge_region = Region {
            x: 100,
            y: 100,
            width: 1820,
            height: 980,
        };
        let (ex, ey, ew, eh) = logical_to_physical_region(&edge_region, 1.0, 1920, 1080).unwrap();
        assert_eq!((ex, ey, ew, eh), (100, 100, 1820, 980));

        // 1 pixel out of bounds
        let out_by_one = Region {
            x: 100,
            y: 100,
            width: 1821,
            height: 980,
        };
        assert!(logical_to_physical_region(&out_by_one, 1.0, 1920, 1080).is_err());

        // Arithmetic overflow safety with u32::MAX width
        let overflow_region = Region {
            x: 10,
            y: 10,
            width: u32::MAX,
            height: 100,
        };
        assert!(logical_to_physical_region(&overflow_region, 1.0, 1920, 1080).is_err());

        // Negative coordinates
        let neg_region = Region {
            x: -1,
            y: 10,
            width: 100,
            height: 100,
        };
        assert!(logical_to_physical_region(&neg_region, 1.0, 1920, 1080).is_err());

        // Invalid scale factors (zero, negative, NaN, infinite)
        assert!(logical_to_physical_region(&region, 0.0, 1920, 1080).is_err());
        assert!(logical_to_physical_region(&region, -1.0, 1920, 1080).is_err());
        assert!(logical_to_physical_region(&region, f32::NAN, 1920, 1080).is_err());
        assert!(logical_to_physical_region(&region, f32::INFINITY, 1920, 1080).is_err());
    }
}
