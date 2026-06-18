mod capture;
mod cli;
mod d3d;
mod output;
mod window_enum;

use clap::Parser;
use cli::{Cli, ExitCode};
use output::{output_error, output_success, MatchedWindow};

type BoxError = Box<dyn std::error::Error>;

fn main() {
    let cli = Cli::parse();

    if cli.help {
        println!("{}", cli::help_text());
        std::process::exit(0);
    }

    if cli.version {
        println!("wgccli 1.0.0");
        std::process::exit(0);
    }

    match run(&cli) {
        Ok(code) => std::process::exit(code as i32),
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(ExitCode::GeneralError as i32);
        }
    }
}

fn run(opts: &Cli) -> Result<ExitCode, BoxError> {
    if opts.list {
        let windows = window_enum::enumerate_windows(opts.include_minimized);
        window_enum::print_window_list(&windows);
        return Ok(ExitCode::Ok);
    }

    if !opts.has_target() {
        output_error(opts, ExitCode::BadArgs, "MISSING_TARGET", "Specify --title, --hwnd, or --list");
        return Ok(ExitCode::BadArgs);
    }

    if opts.out.is_none() {
        output_error(opts, ExitCode::BadArgs, "MISSING_OUTPUT", "Specify --out <dir-or-file>");
        return Ok(ExitCode::BadArgs);
    }

    let target = if let Some(hwnd_val) = opts.hwnd {
        window_enum::find_window_by_hwnd(hwnd_val)
    } else if let Some(ref title) = opts.title {
        let all_windows = window_enum::enumerate_windows(opts.include_minimized);
        window_enum::find_best_match(&all_windows, title, opts.exact)
    } else {
        None
    };

    let target = match target {
        Some(t) => t,
        None => {
            let desc = if let Some(hwnd_val) = opts.hwnd {
                format!("hwnd {hwnd_val}")
            } else {
                format!("title: {}", opts.title.as_deref().unwrap_or(""))
            };
            output_error(
                opts,
                ExitCode::WindowNotFound,
                "WINDOW_NOT_FOUND",
                &format!("No visible top-level window matched {desc}"),
            );
            return Ok(ExitCode::WindowNotFound);
        }
    };

    let win_w = target.rect.right - target.rect.left;
    let win_h = target.rect.bottom - target.rect.top;
    println!("Matched window: {}", target.title);
    println!("Handle: 0x{:016X}", target.hwnd);
    println!("Window size: {win_w}x{win_h}");

    if target.minimized && opts.restore {
        println!("Restoring minimized window...");
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindow(
                windows::Win32::Foundation::HWND(target.hwnd as *mut _),
                windows::Win32::UI::WindowsAndMessaging::SW_RESTORE,
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
        for _ in 0..10 {
            let is_iconic = unsafe {
                windows::Win32::UI::WindowsAndMessaging::IsIconic(
                    windows::Win32::Foundation::HWND(target.hwnd as *mut _),
                )
            };
            if !is_iconic.as_bool() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    let d3d_res = d3d::create_d3d_device()?;
    if d3d_res.is_none() {
        output_error(opts, ExitCode::WgcUnavailable, "D3D11_INIT_FAILED", "Failed to create D3D11 device");
        return Ok(ExitCode::WgcUnavailable);
    }
    let d3d_res = d3d_res.unwrap();

    let captured = capture::capture_window(&d3d_res.device, target.hwnd, opts.timeout_ms)?;
    if captured.is_none() {
        output_error(opts, ExitCode::CaptureFailed, "CAPTURE_FAILED", "Failed to capture window");
        return Ok(ExitCode::CaptureFailed);
    }
    let captured = captured.unwrap();

    let out_dir = opts.out.as_ref().unwrap();
    let out_path = if out_dir.to_lowercase().ends_with(".png") {
        let parent = std::path::Path::new(out_dir).parent().unwrap_or(std::path::Path::new("."));
        std::fs::create_dir_all(parent)?;
        out_dir.clone()
    } else {
        std::fs::create_dir_all(out_dir)?;
        generate_png_path(out_dir, &target.title)
    };

    if let Err(e) = capture::save_png(&captured, &out_path) {
        output_error(opts, ExitCode::SaveFailed, "SAVE_FAILED", &format!("Failed to save PNG to {out_path}: {e}"));
        return Ok(ExitCode::SaveFailed);
    }

    let mw = MatchedWindow {
        title: target.title.clone(),
        class_name: target.class_name.clone(),
        hwnd: target.hwnd,
        pid: target.pid,
        width: captured.width,
        height: captured.height,
        minimized: target.minimized,
    };

    output_success(opts, &mw, &out_path);
    Ok(ExitCode::Ok)
}

fn generate_png_path(dir: &str, title: &str) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let secs_in_day = now % 86400;
    let hours = secs_in_day / 3600;
    let minutes = (secs_in_day % 3600) / 60;
    let seconds = secs_in_day % 60;
    let days = now / 86400 + 719468;
    let era = days / 146097;
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    let sanitized = sanitize_title(title);
    let filename = format!("{:04}{:02}{:02}-{:02}{:02}{:02}-{sanitized}.png", y, m, d, hours, minutes, seconds);
    std::path::Path::new(dir)
        .join(filename)
        .to_string_lossy()
        .into_owned()
}

fn sanitize_title(title: &str) -> String {
    let mut result: String = title
        .chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            _ => c,
        })
        .collect();

    result = result.trim().to_string();
    if result.len() > 80 {
        result.truncate(80);
    }
    if result.is_empty() {
        result = "window".to_string();
    }
    result
}
