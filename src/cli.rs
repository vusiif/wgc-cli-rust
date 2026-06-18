use clap::Parser;

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Ok = 0,
    GeneralError = 1,
    BadArgs = 2,
    WindowNotFound = 3,
    WgcUnavailable = 4,
    CaptureFailed = 5,
    SaveFailed = 6,
    #[allow(dead_code)]
    Timeout = 7,
    #[allow(dead_code)]
    PermissionDenied = 8,
}

#[derive(Parser, Debug, Clone)]
#[command(name = "wgccli", about = "Windows Graphics Capture command-line tool", disable_help_flag = true)]
pub struct Cli {
    #[arg(long = "title")]
    pub title: Option<String>,

    #[arg(long = "exact", default_value_t = false)]
    pub exact: bool,

    #[arg(long = "hwnd", value_parser = parse_hwnd)]
    pub hwnd: Option<u64>,

    #[arg(long = "list", default_value_t = false)]
    pub list: bool,

    #[arg(long = "out")]
    pub out: Option<String>,

    #[arg(long = "json", default_value_t = false)]
    pub json: bool,

    #[arg(long = "timeout-ms", default_value_t = 3000)]
    pub timeout_ms: u32,

    #[arg(long = "include-minimized", default_value_t = true)]
    pub include_minimized: bool,

    #[arg(long = "no-restore", default_value_t = false)]
    pub no_restore: bool,

    #[arg(long = "restore", default_value_t = true)]
    pub restore: bool,

    #[arg(long = "help", default_value_t = false)]
    pub help: bool,

    #[arg(long = "version", default_value_t = false)]
    pub version: bool,
}

impl Cli {
    pub fn has_target(&self) -> bool {
        self.title.is_some() || self.hwnd.is_some()
    }
}

fn parse_hwnd(s: &str) -> Result<u64, String> {
    let s = s.trim();
    let val = if s.len() > 2 && (s.starts_with("0x") || s.starts_with("0X")) {
        u64::from_str_radix(&s[2..], 16)
    } else {
        s.parse::<u64>()
    };
    val.map_err(|e| format!("invalid HWND value: {s}: {e}")).and_then(|v| {
        if v == 0 {
            Err(format!("invalid HWND value: {s}"))
        } else {
            Ok(v)
        }
    })
}

pub fn help_text() -> &'static str {
    "\
wgccli - Windows Graphics Capture command-line tool

Usage:
  wgccli.exe --title <text> [--exact] --out <dir-or-file> [options]
  wgccli.exe --hwnd <hex-or-decimal> --out <dir-or-file> [options]
  wgccli.exe --list

Required (one of):
  --title <text>             Match window by title (partial, case-insensitive)
  --hwnd <hex-or-decimal>    Capture by window handle
  --list                     List candidate windows

Options:
  --exact                    Exact title match (case-insensitive)
  --out <dir-or-file>        Output directory or full PNG file path
  --json                     JSON output
  --timeout-ms <number>      Wait for first frame timeout (default: 3000)
  --include-minimized        Include minimized windows in list (default: true)
  --no-restore               Do not try to restore minimized window
  --restore                  Try to restore window before capture (default: true)
  --help                     Print this help
  --version                  Print version

Examples:
  wgccli.exe --title \"Notepad\" --out C:\\Temp
  wgccli.exe --title \"Android Studio\" --exact --out C:\\Screenshots
  wgccli.exe --hwnd 0x00123456 --out C:\\Screenshots --json
  wgccli.exe --list"
}
