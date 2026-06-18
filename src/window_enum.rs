use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowLongW, GetWindowRect, GetWindowTextLengthW,
    GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible,
    GWL_EXSTYLE, WS_EX_TOOLWINDOW,
};

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub hwnd: i64,
    pub pid: u32,
    pub title: String,
    pub class_name: String,
    pub rect: RECT,
    pub visible: bool,
    pub minimized: bool,
}

const NOISE_CLASSES: &[&str] = &[
    "IME", "MSCTFIME UI", "SoPY_Hint", "Sogou_TSF_UI", "CiceroUIWndFrame",
    "TF_FloatingLangBar_WndTitle", ".NET-BroadcastEventWindow", "GDI+ Hook Window Class",
    "OleDdeWndClass", "COMTASKSWINDOWCLASS", "DDE Server Window", "Default IME",
    "Progman", "Shell_TrayWnd", "Windows.UI.Core.CoreWindow", "FilterGraphWindow",
    "ActiveMovie Window", "MediaContextNotificationWindow", "SystemResourceNotifyWindow",
    "BroadcastListenerWindow",
];

const NOISE_TITLES: &[&str] = &[
    "Microsoft Text Input Application",
    "Program Manager",
    "Default IME",
    "Hidden Window",
];

fn is_noise_window(w: &WindowInfo) -> bool {
    if !w.visible {
        return true;
    }
    let width = w.rect.right - w.rect.left;
    let height = w.rect.bottom - w.rect.top;
    if width <= 1 && height <= 1 && !w.minimized {
        return true;
    }
    for nc in NOISE_CLASSES {
        if w.class_name == *nc {
            return true;
        }
    }
    if w.class_name.contains("HwndWrapper[OMEN") && w.title.contains("Hidden") {
        return true;
    }
    for nt in NOISE_TITLES {
        if w.title == *nt {
            return true;
        }
    }
    let ex_style = unsafe { GetWindowLongW(HWND(w.hwnd as *mut _), GWL_EXSTYLE) };
    if ex_style & WS_EX_TOOLWINDOW.0 as i32 != 0 {
        return true;
    }
    false
}

unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows = &mut *(lparam.0 as *mut Vec<WindowInfo>);

    let title_len = GetWindowTextLengthW(hwnd);
    if title_len == 0 {
        return BOOL(1);
    }

    let mut title_buf = vec![0u16; (title_len + 1) as usize];
    GetWindowTextW(hwnd, &mut title_buf);
    title_buf.truncate(title_len as usize);
    let title = String::from_utf16_lossy(&title_buf);

    let mut class_buf = [0u16; 256];
    let class_len = GetClassNameW(hwnd, &mut class_buf);
    let class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]);

    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));

    let mut rect = RECT::default();
    let _ = GetWindowRect(hwnd, &mut rect);

    let visible = IsWindowVisible(hwnd).as_bool();
    let minimized = IsIconic(hwnd).as_bool();

    windows.push(WindowInfo {
        hwnd: hwnd.0 as i64,
        pid,
        title,
        class_name,
        rect,
        visible,
        minimized,
    });

    BOOL(1)
}

pub fn enumerate_windows(_include_minimized: bool) -> Vec<WindowInfo> {
    let mut windows: Vec<WindowInfo> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM(&mut windows as *mut _ as isize));
    }
    windows
}

pub fn find_window_by_hwnd(hwnd_value: u64) -> Option<WindowInfo> {
    let hwnd = HWND(hwnd_value as *mut _);
    if !unsafe { IsWindow(hwnd) }.as_bool() {
        return None;
    }

    let visible = unsafe { IsWindowVisible(hwnd) }.as_bool();
    let minimized = unsafe { IsIconic(hwnd) }.as_bool();

    let title_len = unsafe { GetWindowTextLengthW(hwnd) };
    let title = if title_len > 0 {
        let mut buf = vec![0u16; (title_len + 1) as usize];
        unsafe { GetWindowTextW(hwnd, &mut buf) };
        buf.truncate(title_len as usize);
        String::from_utf16_lossy(&buf)
    } else {
        String::new()
    };

    let mut class_buf = [0u16; 256];
    let class_len = unsafe { GetClassNameW(hwnd, &mut class_buf) };
    let class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]);

    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };

    let mut rect = RECT::default();
    unsafe { let _ = GetWindowRect(hwnd, &mut rect); }

    Some(WindowInfo {
        hwnd: hwnd.0 as i64,
        pid,
        title,
        class_name,
        rect,
        visible,
        minimized,
    })
}

pub fn find_best_match(windows: &[WindowInfo], query: &str, exact: bool) -> Option<WindowInfo> {
    let q_lower = query.to_lowercase();

    let mut candidates: Vec<(&WindowInfo, i32, usize)> = Vec::new();

    for w in windows {
        if is_noise_window(w) {
            continue;
        }
        let t_lower = w.title.to_lowercase();

        if exact {
            if t_lower == q_lower {
                candidates.push((w, 100, w.title.len()));
            }
        } else if t_lower == q_lower {
            candidates.push((w, 100, w.title.len()));
        } else if t_lower.starts_with(&q_lower) {
            candidates.push((w, 80, w.title.len()));
        } else if t_lower.contains(&q_lower) {
            candidates.push((w, 60, w.title.len()));
        }
    }

    candidates.sort_by(|a, b| {
        b.1.cmp(&a.1).then(a.2.cmp(&b.2))
    });

    candidates.first().map(|(w, _, _)| (*w).clone())
}

pub fn filter_windows(
    windows: &[WindowInfo],
    pid: Option<u32>,
    process: Option<&str>,
    class_name: Option<&str>,
) -> Vec<WindowInfo> {
    windows.iter().filter(|w| {
        if !w.visible {
            return false;
        }
        let width = w.rect.right - w.rect.left;
        let height = w.rect.bottom - w.rect.top;
        if width <= 1 && height <= 1 && !w.minimized {
            return false;
        }
        if let Some(pid_filter) = pid {
            if w.pid != pid_filter {
                return false;
            }
        }
        if let Some(proc_filter) = process {
            if !w.title.to_lowercase().contains(&proc_filter.to_lowercase()) {
                return false;
            }
        }
        if let Some(cls_filter) = class_name {
            if !w.class_name.to_lowercase().contains(&cls_filter.to_lowercase()) {
                return false;
            }
        }
        true
    }).cloned().collect()
}

pub fn print_window_list(windows: &[WindowInfo]) {
    println!(
        "{:<20} {:<8} {:<7} {:<7} {:<12} {:<24} Title",
        "HandleHex", "PID", "Width", "Height", "State", "ClassName"
    );
    println!("{}", "-".repeat(90));

    let mut shown = 0;
    for w in windows {
        if is_noise_window(w) {
            continue;
        }
        let width = w.rect.right - w.rect.left;
        let height = w.rect.bottom - w.rect.top;
        let state = if w.minimized { "Minimized" } else { "Normal" };
        let cls = if w.class_name.len() > 22 {
            &w.class_name[..22]
        } else {
            &w.class_name
        };
        println!(
            "0x{:016X} {:<8} {:<7} {:<7} {:<12} {:<24} {}",
            w.hwnd, w.pid, width, height, state, cls, w.title
        );
        shown += 1;
    }

    println!("\nShowing {shown} of {} windows", windows.len());
}
