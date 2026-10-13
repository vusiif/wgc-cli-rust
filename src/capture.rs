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
pub fn logical_to_physical_region(
    region: &Region,
    scale_factor: f32,
    monitor_width: u32,
    monitor_height: u32,
) -> Result<(u32, u32, u32, u32), CaptureError> {
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

    // Scale logical coordinates by DPI scale factor
    let phys_x = (region.x as f32 * scale_factor).round() as u32;
    let phys_y = (region.y as f32 * scale_factor).round() as u32;
    let phys_w = (region.width as f32 * scale_factor).round() as u32;
    let phys_h = (region.height as f32 * scale_factor).round() as u32;

    debug!(
        "Translated region: logical({:?}) * scale({}) -> physical(x={}, y={}, w={}, h={})",
        region, scale_factor, phys_x, phys_y, phys_w, phys_h
    );

    if phys_x + phys_w > monitor_width || phys_y + phys_h > monitor_height || phys_w == 0 || phys_h == 0 {
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

        // Out of bounds test
        let out_region = Region {
            x: 1800,
            y: 1000,
            width: 500,
            height: 500,
        };
        assert!(logical_to_physical_region(&out_region, 1.0, 1920, 1080).is_err());
    }
}
