use std::io::Write;
use windows::Win32::Graphics::Direct3D11::ID3D11Device;

type BoxError = Box<dyn std::error::Error>;

pub struct CapturedImage {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub bgra: Vec<u8>,
}

#[repr(C)]
struct WgcCaptureResult {
    width: u32,
    height: u32,
    stride: u32,
    data: *mut u8,
    data_len: u32,
    success: bool,
    error_code: u32,
}

extern "C" {
    fn wgc_capture_window(
        device: *mut std::ffi::c_void,
        hwnd: u64,
        timeout_ms: u32,
    ) -> WgcCaptureResult;

    fn wgc_free_capture(result: *mut WgcCaptureResult);
}

pub fn capture_window(
    device: &ID3D11Device,
    hwnd: i64,
    timeout_ms: u32,
) -> Result<Option<CapturedImage>, BoxError> {
    unsafe {
        let _ = windows::Win32::System::WinRT::RoInitialize(
            windows::Win32::System::WinRT::RO_INIT_MULTITHREADED,
        );
    }

    let device_raw = windows::core::Interface::as_raw(device) as *mut std::ffi::c_void;

    let mut cpp_result = unsafe { wgc_capture_window(device_raw, hwnd as u64, timeout_ms) };

    if !cpp_result.success {
        return Err(format!("WGC capture failed with error code: {}", cpp_result.error_code).into());
    }

    let img = CapturedImage {
        width: cpp_result.width,
        height: cpp_result.height,
        stride: cpp_result.stride,
        bgra: unsafe {
            std::slice::from_raw_parts(cpp_result.data, cpp_result.data_len as usize).to_vec()
        },
    };

    unsafe { wgc_free_capture(&mut cpp_result) };

    Ok(Some(img))
}

pub fn save_png(image: &CapturedImage, path: &str) -> Result<(), BoxError> {
    let file = std::fs::File::create(path)?;
    let mut writer = std::io::BufWriter::new(file);

    let width = image.width;
    let height = image.height;

    writer.write_all(&[137, 80, 78, 71, 13, 10, 26, 10])?;

    let ihdr_data = {
        let mut d = Vec::with_capacity(13);
        d.extend_from_slice(&width.to_be_bytes());
        d.extend_from_slice(&height.to_be_bytes());
        d.push(8);
        d.push(6);
        d.push(0);
        d.push(0);
        d.push(0);
        d
    };
    write_chunk(&mut writer, b"IHDR", &ihdr_data)?;

    let mut raw_data = Vec::with_capacity((width as usize * 4 + 1) * height as usize);
    for y in 0..height as usize {
        raw_data.push(0);
        let row_start = y * image.stride as usize;
        let row_end = row_start + width as usize * 4;
        for x in (row_start..row_end).step_by(4) {
            raw_data.push(image.bgra[x + 2]);
            raw_data.push(image.bgra[x + 1]);
            raw_data.push(image.bgra[x]);
            raw_data.push(image.bgra[x + 3]);
        }
    }

    let compressed = deflate_compress(&raw_data)?;
    write_chunk(&mut writer, b"IDAT", &compressed)?;

    write_chunk(&mut writer, b"IEND", &[])?;

    writer.flush()?;
    Ok(())
}

fn write_chunk(writer: &mut impl std::io::Write, chunk_type: &[u8; 4], data: &[u8]) -> Result<(), BoxError> {
    writer.write_all(&(data.len() as u32).to_be_bytes())?;
    writer.write_all(chunk_type)?;
    writer.write_all(data)?;

    let mut crc = Crc32::new();
    crc.update(chunk_type);
    crc.update(data);
    writer.write_all(&crc.finalize().to_be_bytes())?;
    Ok(())
}

struct Crc32 {
    value: u32,
}

impl Crc32 {
    fn new() -> Self {
        Self { value: 0xFFFFFFFF }
    }

    fn update(&mut self, data: &[u8]) {
        for &byte in data {
            self.value ^= byte as u32;
            for _ in 0..8 {
                if self.value & 1 != 0 {
                    self.value = (self.value >> 1) ^ 0xEDB88320;
                } else {
                    self.value >>= 1;
                }
            }
        }
    }

    fn finalize(self) -> u32 {
        self.value ^ 0xFFFFFFFF
    }
}

fn deflate_compress(data: &[u8]) -> Result<Vec<u8>, BoxError> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(data)?;
    let compressed = encoder.finish()?;
    Ok(compressed)
}
