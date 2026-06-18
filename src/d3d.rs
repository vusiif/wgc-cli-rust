use anyhow::Result;
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_FLAG, D3D11CreateDevice, D3D11_SDK_VERSION, ID3D11Device,
};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::core::Interface;

pub struct D3DResources {
    pub device: ID3D11Device,
    #[allow(dead_code)]
    pub dxgi_device: IDXGIDevice,
}

pub fn create_d3d_device() -> Result<Option<D3DResources>> {
    let flags = D3D11_CREATE_DEVICE_FLAG(0x20); // D3D11_CREATE_DEVICE_BGRA_SUPPORT
    let feature_levels = [D3D_FEATURE_LEVEL_11_0];

    let mut device = None;
    let mut context = None;
    let mut feature_level = D3D_FEATURE_LEVEL_11_0;

    let hr = unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            None,
            flags,
            Some(&feature_levels),
            D3D11_SDK_VERSION,
            Some(&mut device),
            Some(&mut feature_level),
            Some(&mut context),
        )
    };

    if hr.is_err() {
        return Ok(None);
    }

    let device = device.ok_or_else(|| anyhow::anyhow!("D3D11 device creation returned null"))?;

    let dxgi_device: IDXGIDevice = device.cast()?;

    Ok(Some(D3DResources { device, dxgi_device }))
}
