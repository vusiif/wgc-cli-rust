use serde::{Deserialize, Serialize};

/// Comprehensive diagnostic report of environment, hardware displays, and graphics capture capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorReport {
    pub os: String,
    pub arch: String,
    pub backend: String,
    pub monitors_detected: usize,
    pub primary_monitor: Option<String>,
    pub clipboard_available: bool,
    pub supported_formats: Vec<String>,
    pub status: String,
}

/// Run environment diagnostics and assemble capability report.
pub fn run_doctor() -> DoctorReport {
    let os = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();

    let backend = if cfg!(target_os = "windows") {
        "Windows Graphics Capture (WGC) / DXGI / GDI".to_string()
    } else if cfg!(target_os = "macos") {
        "CoreGraphics / ScreenCaptureKit".to_string()
    } else {
        "Wayland / X11 (via xcap)".to_string()
    };

    let (monitors_detected, primary_monitor) = match crate::capture::get_monitor_infos() {
        Ok(monitors) => {
            let count = monitors.len();
            let primary = monitors
                .iter()
                .find(|m| m.is_primary)
                .or_else(|| monitors.first())
                .map(|m| {
                    format!(
                        "[{}] {} - {}x{} @ {:.2}x",
                        m.index, m.name, m.width, m.height, m.scale_factor
                    )
                });
            (count, primary)
        }
        Err(_) => (0, None),
    };

    let clipboard_available = arboard::Clipboard::new().is_ok();

    let supported_formats = vec![
        "PNG (Lossless)".to_string(),
        "JPEG (Standard)".to_string(),
        "WebP (High-efficiency native)".to_string(),
    ];

    let status = if monitors_detected > 0 {
        "Operational".to_string()
    } else {
        "Degraded (No active displays detected)".to_string()
    };

    DoctorReport {
        os,
        arch,
        backend,
        monitors_detected,
        primary_monitor,
        clipboard_available,
        supported_formats,
        status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_doctor_fields() {
        let report = run_doctor();
        assert!(!report.os.is_empty());
        assert!(!report.arch.is_empty());
        assert!(!report.backend.is_empty());
        assert!(!report.supported_formats.is_empty());
        assert!(!report.status.is_empty());
    }
}
