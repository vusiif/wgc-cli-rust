use windows::Win32::Storage::FileSystem::{ReadFile, WriteFile, FlushFileBuffers, FILE_SHARE_MODE, FILE_FLAGS_AND_ATTRIBUTES, OPEN_EXISTING};

pub fn run_client(pipe_name: &str, request_json: &str) -> i32 {
    let full_pipe = format!("\\\\.\\pipe\\{pipe_name}");

    unsafe {
        let pipe_w = windows::core::HSTRING::from(&full_pipe[..]);
        let h_pipe = windows::Win32::Storage::FileSystem::CreateFileW(
            &pipe_w,
            0x80000000 | 0x40000000, // GENERIC_READ | GENERIC_WRITE
            FILE_SHARE_MODE(0),
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        );

        let h_pipe = match h_pipe {
            Ok(h) => h,
            Err(_) => {
                eprintln!("[error] Cannot connect to pipe {full_pipe}. Is wgccli --server running?");
                return 1;
            }
        };

        // Convert request to UTF-16LE
        let request_utf16: Vec<u16> = request_json.encode_utf16().collect();
        let request_bytes = std::slice::from_raw_parts(
            request_utf16.as_ptr() as *const u8,
            request_utf16.len() * 2,
        );

        let mut bytes_written = 0u32;
        if WriteFile(h_pipe, Some(request_bytes), Some(&mut bytes_written), None).is_err() {
            eprintln!("[error] WriteFile failed");
            let _ = windows::Win32::Foundation::CloseHandle(h_pipe);
            return 1;
        }
        let _ = FlushFileBuffers(h_pipe);

        // Read response
        let mut buf = [0u8; 65536];
        let mut bytes_read = 0u32;
        if ReadFile(h_pipe, Some(&mut buf), Some(&mut bytes_read), None).is_err() || bytes_read == 0 {
            eprintln!("[error] ReadFile failed");
            let _ = windows::Win32::Foundation::CloseHandle(h_pipe);
            return 1;
        }

        let response = if bytes_read >= 2 {
            let u16_slice = std::slice::from_raw_parts(
                buf.as_ptr() as *const u16,
                bytes_read as usize / 2,
            );
            String::from_utf16_lossy(u16_slice)
        } else {
            String::new()
        };

        println!("{response}");
        let _ = windows::Win32::Foundation::CloseHandle(h_pipe);
    }

    0
}
