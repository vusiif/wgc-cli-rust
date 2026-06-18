use crate::cli::{Cli, ExitCode};

pub struct MatchedWindow {
    pub title: String,
    pub class_name: String,
    pub hwnd: i64,
    pub pid: u32,
    pub width: u32,
    pub height: u32,
    pub minimized: bool,
}

fn hwnd_to_hex(hwnd: i64) -> String {
    format!("0x{hwnd:016X}")
}

pub fn escape_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

fn state_str(minimized: bool) -> &'static str {
    if minimized {
        "minimized"
    } else {
        "normal"
    }
}

pub fn output_success(opts: &Cli, win: &MatchedWindow, screenshot_path: &str) {
    if opts.json {
        println!(
            "{{\n  \"ok\": true,\n  \"matchedWindow\": {{\n    \"title\": \"{}\",\n    \"hwnd\": \"{}\",\n    \"pid\": {},\n    \"width\": {},\n    \"height\": {},\n    \"className\": \"{}\",\n    \"state\": \"{}\"\n  }},\n  \"screenshotPath\": \"{}\"\n}}",
            escape_json_string(&win.title),
            hwnd_to_hex(win.hwnd),
            win.pid,
            win.width,
            win.height,
            escape_json_string(&win.class_name),
            state_str(win.minimized),
            escape_json_string(screenshot_path),
        );
    } else {
        println!("Matched window: {}", win.title);
        println!("Handle: {}", hwnd_to_hex(win.hwnd));
        println!("Window size: {}x{}", win.width, win.height);
        println!("RESULT_SCREENSHOT_PATH={screenshot_path}");
    }
}

pub fn output_error(opts: &Cli, _code: ExitCode, error_code: &str, message: &str) {
    if opts.json {
        println!(
            "{{\n  \"ok\": false,\n  \"errorCode\": \"{}\",\n  \"message\": \"{}\"\n}}",
            escape_json_string(error_code),
            escape_json_string(message),
        );
    } else {
        eprintln!("ERROR_CODE={error_code}");
        eprintln!("ERROR_MESSAGE={message}");
    }
}
