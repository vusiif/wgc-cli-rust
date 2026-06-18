use crate::{capture, d3d, window_enum};
use windows::Win32::Storage::FileSystem::{ReadFile, WriteFile, FlushFileBuffers, FILE_FLAGS_AND_ATTRIBUTES};
use windows::Win32::System::Pipes::{CreateNamedPipeW, ConnectNamedPipe, DisconnectNamedPipe, NAMED_PIPE_MODE};

fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => {
                if c < '\x20' {
                    out.push_str(&format!("\\u{:04x}", c as u32));
                } else {
                    out.push(c);
                }
            }
        }
    }
    out
}

fn hwnd_hex(hwnd: i64) -> String {
    format!("0x{hwnd:016X}")
}

fn handle_list() -> String {
    let windows = window_enum::enumerate_windows(true);
    let mut items = Vec::new();
    for w in &windows {
        if !w.visible {
            continue;
        }
        let ww = w.rect.right - w.rect.left;
        let wh = w.rect.bottom - w.rect.top;
        if ww <= 1 && wh <= 1 && !w.minimized {
            continue;
        }
        items.push(format!(
            "{{\"hwnd\":\"{}\",\"pid\":{},\"title\":\"{}\",\"className\":\"{}\",\"width\":{},\"height\":{},\"minimized\":{}}}",
            hwnd_hex(w.hwnd),
            w.pid,
            escape_json(&w.title),
            escape_json(&w.class_name),
            ww,
            wh,
            if w.minimized { "true" } else { "false" }
        ));
    }
    format!("{{\"ok\":true,\"windows\":[{}]}}", items.join(","))
}

fn handle_capture(request: &str) -> String {
    let title = json_get_string(request, "title");
    let out = json_get_string(request, "out").unwrap_or_else(|| ".".to_string());
    let hwnd_val = json_get_u64(request, "hwnd");
    let exact = json_get_bool(request, "exact");
    let pid = json_get_u64(request, "pid").map(|v| v as u32);
    let process = json_get_string(request, "process");
    let class_name = json_get_string(request, "className");

    let target = if let Some(hwnd_val) = hwnd_val {
        if hwnd_val != 0 {
            window_enum::find_window_by_hwnd(hwnd_val)
        } else {
            None
        }
    } else {
        let windows = window_enum::enumerate_windows(true);
        let filtered = window_enum::filter_windows(&windows, pid, process.as_deref(), class_name.as_deref());
        if let Some(ref t) = title {
            window_enum::find_best_match(&filtered, t, exact)
        } else if !filtered.is_empty() {
            Some(filtered[0].clone())
        } else {
            None
        }
    };

    let target = match target {
        Some(t) => t,
        None => return r#"{"ok":false,"errorCode":"WINDOW_NOT_FOUND","message":"No window matched"}"#.to_string(),
    };

    // Restore if minimized
    if target.minimized {
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindow(
                windows::Win32::Foundation::HWND(target.hwnd as *mut _),
                windows::Win32::UI::WindowsAndMessaging::SW_RESTORE,
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    let d3d_res = match d3d::create_d3d_device() {
        Ok(Some(d)) => d,
        _ => return r#"{"ok":false,"errorCode":"D3D11_INIT_FAILED","message":"D3D11 device creation failed"}"#.to_string(),
    };

    let captured = match capture::capture_window(&d3d_res.device, target.hwnd, 3000) {
        Ok(Some(img)) => img,
        _ => return r#"{"ok":false,"errorCode":"CAPTURE_FAILED","message":"Capture failed"}"#.to_string(),
    };

    let out_path = if out.to_lowercase().ends_with(".png") {
        let parent = std::path::Path::new(&out).parent().unwrap_or(std::path::Path::new("."));
        let _ = std::fs::create_dir_all(parent);
        out.clone()
    } else {
        let _ = std::fs::create_dir_all(&out);
        crate::generate_png_path(&out, &target.title)
    };

    if capture::save_png(&captured, &out_path).is_err() {
        return r#"{"ok":false,"errorCode":"SAVE_FAILED","message":"PNG save failed"}"#.to_string();
    }

    format!(
        "{{\"ok\":true,\"matchedWindow\":{{\"title\":\"{}\",\"hwnd\":\"{}\",\"pid\":{},\"width\":{},\"height\":{},\"minimized\":{}}},\"screenshotPath\":\"{}\"}}",
        escape_json(&target.title),
        hwnd_hex(target.hwnd),
        target.pid,
        target.rect.right - target.rect.left,
        target.rect.bottom - target.rect.top,
        if target.minimized { "true" } else { "false" },
        escape_json(&out_path)
    )
}

fn process_request(request: &str) -> String {
    let action = json_get_string(request, "action").unwrap_or_default();
    match action.as_str() {
        "list" => handle_list(),
        "capture" => handle_capture(request),
        _ => r#"{"ok":false,"errorCode":"BAD_ACTION","message":"Unknown action"}"#.to_string(),
    }
}

pub fn run_server(pipe_name: &str) -> i32 {
    let full_pipe = format!("\\\\.\\pipe\\{pipe_name}");
    eprintln!("[server] Listening on {full_pipe}");
    eprintln!("[server] Agent connects via: wgccli.exe --client <json>");
    eprintln!("[server] Press Ctrl+C to stop.");

    loop {
        unsafe {
            let pipe_w = windows::core::HSTRING::from(&full_pipe[..]);
            let h_pipe = CreateNamedPipeW(
                &pipe_w,
                FILE_FLAGS_AND_ATTRIBUTES(0x40000000 | 3), // FILE_FLAG_OVERLAPPED | PIPE_ACCESS_DUPLEX
                NAMED_PIPE_MODE(0x00000002 | 0x00000004), // PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE
                0xFF, // PIPE_UNLIMITED_INSTANCES
                65536,
                65536,
                0,
                None,
            );

            if h_pipe == windows::Win32::Foundation::INVALID_HANDLE_VALUE {
                eprintln!("[server] CreateNamedPipe failed");
                return 1;
            }

            if ConnectNamedPipe(h_pipe, None).is_err() {
                let _ = windows::Win32::Foundation::CloseHandle(h_pipe);
                continue;
            }

            // Read request
            let mut buf = [0u8; 4096];
            let mut bytes_read = 0u32;
            if ReadFile(h_pipe, Some(&mut buf), Some(&mut bytes_read), None).is_err() || bytes_read == 0 {
                let _ = DisconnectNamedPipe(h_pipe);
                let _ = windows::Win32::Foundation::CloseHandle(h_pipe);
                continue;
            }

            // Convert from UTF-16LE
            let request = if bytes_read >= 2 {
                let u16_slice = std::slice::from_raw_parts(
                    buf.as_ptr() as *const u16,
                    bytes_read as usize / 2,
                );
                String::from_utf16_lossy(u16_slice)
            } else {
                String::new()
            };

            let response = process_request(&request);

            // Convert response to UTF-16LE
            let response_utf16: Vec<u16> = response.encode_utf16().collect();
            let response_bytes = std::slice::from_raw_parts(
                response_utf16.as_ptr() as *const u8,
                response_utf16.len() * 2,
            );

            let mut bytes_written = 0u32;
            let _ = WriteFile(h_pipe, Some(response_bytes), Some(&mut bytes_written), None);
            let _ = FlushFileBuffers(h_pipe);
            let _ = DisconnectNamedPipe(h_pipe);
            let _ = windows::Win32::Foundation::CloseHandle(h_pipe);
        }
    }
}

// Simple JSON helpers (no external dependency)
fn json_get_string(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":");
    let pos = json.find(&needle)?;
    let rest = &json[pos + needle.len()..].trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let rest = &rest[1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn json_get_u64(json: &str, key: &str) -> Option<u64> {
    let needle = format!("\"{key}\":");
    let pos = json.find(&needle)?;
    let rest = &json[pos + needle.len()..].trim_start();
    rest.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
}

fn json_get_bool(json: &str, key: &str) -> bool {
    let needle = format!("\"{key}\":");
    if let Some(pos) = json.find(&needle) {
        let rest = &json[pos + needle.len()..].trim_start();
        rest.starts_with("true")
    } else {
        false
    }
}
