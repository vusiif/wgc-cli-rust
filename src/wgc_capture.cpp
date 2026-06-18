#include <windows.h>
#include <d3d11.h>
#include <dxgi1_2.h>
#include <wrl/client.h>

#define _SILENCE_EXPERIMENTAL_COROUTINE_DEPRECATION_WARNINGS
#include <winrt/base.h>
#include <winrt/Windows.Foundation.h>
#include <winrt/Windows.Graphics.Capture.h>
#include <winrt/Windows.Graphics.DirectX.h>
#include <winrt/Windows.Graphics.DirectX.Direct3D11.h>

#include <Windows.Graphics.Capture.Interop.h>
#include <windows.graphics.directx.direct3d11.interop.h>

using Microsoft::WRL::ComPtr;
namespace wgc = winrt::Windows::Graphics::Capture;
namespace wgdx = winrt::Windows::Graphics::DirectX;
namespace wgdxdd = winrt::Windows::Graphics::DirectX::Direct3D11;

struct CapturedImageData {
    uint32_t width;
    uint32_t height;
    uint32_t stride;
    uint8_t* bgra;
};

static wgdxdd::IDirect3DDevice make_winrt_device(ID3D11Device* device) {
    ComPtr<IDXGIDevice> dxgi_device;
    HRESULT hr = device->QueryInterface(IID_PPV_ARGS(&dxgi_device));
    if (FAILED(hr)) return nullptr;

    wgdxdd::IDirect3DDevice rt_device{ nullptr };
    hr = CreateDirect3D11DeviceFromDXGIDevice(dxgi_device.Get(),
        reinterpret_cast<IInspectable**>(winrt::put_abi(rt_device)));
    if (FAILED(hr)) return nullptr;

    return rt_device;
}

static ComPtr<ID3D11Texture2D> get_texture_from_surface(wgdxdd::IDirect3DSurface surface) {
    auto* raw = static_cast<IUnknown*>(winrt::get_abi(surface));
    if (!raw) return nullptr;

    ::Windows::Graphics::DirectX::Direct3D11::IDirect3DDxgiInterfaceAccess* dxgi_access = nullptr;
    HRESULT hr = raw->QueryInterface(
        __uuidof(::Windows::Graphics::DirectX::Direct3D11::IDirect3DDxgiInterfaceAccess),
        reinterpret_cast<void**>(&dxgi_access));

    if (FAILED(hr)) return nullptr;

    ComPtr<ID3D11Texture2D> texture;
    hr = dxgi_access->GetInterface(IID_PPV_ARGS(&texture));
    dxgi_access->Release();
    return SUCCEEDED(hr) ? texture : nullptr;
}

extern "C" {

struct WgcCaptureResult {
    uint32_t width;
    uint32_t height;
    uint32_t stride;
    uint8_t* data;
    uint32_t data_len;
    bool success;
    uint32_t error_code;
};

WgcCaptureResult wgc_capture_window(ID3D11Device* device, uint64_t hwnd_value, uint32_t timeout_ms) {
    WgcCaptureResult result = {};

    try {
        winrt::init_apartment(winrt::apartment_type::single_threaded);
    } catch (...) {}

    try {
        auto rt_device = make_winrt_device(device);
        if (!rt_device) {
            result.error_code = 1;
            return result;
        }

        auto interop = winrt::get_activation_factory<wgc::GraphicsCaptureItem>()
            .as<IGraphicsCaptureItemInterop>();

        wgc::GraphicsCaptureItem item{ nullptr };
        HWND hwnd = reinterpret_cast<HWND>(static_cast<uintptr_t>(hwnd_value));
        HRESULT hr = interop->CreateForWindow(hwnd,
            winrt::guid_of<wgc::GraphicsCaptureItem>(),
            winrt::put_abi(item));

        if (FAILED(hr) || !item) {
            result.error_code = 2;
            return result;
        }

        auto size = item.Size();
        auto frame_pool = wgc::Direct3D11CaptureFramePool::CreateFreeThreaded(
            rt_device,
            wgdx::DirectXPixelFormat::B8G8R8A8UIntNormalized,
            1,
            size);

        auto session = frame_pool.CreateCaptureSession(item);
        try { session.IsCursorCaptureEnabled(false); } catch (...) {}

        winrt::handle frame_event{ CreateEvent(nullptr, TRUE, FALSE, nullptr) };
        CapturedImageData img = {};

        frame_pool.FrameArrived([&](wgc::Direct3D11CaptureFramePool const& pool,
                                     winrt::Windows::Foundation::IInspectable const&) {
            try {
                auto frame = pool.TryGetNextFrame();
                if (!frame) {
                    SetEvent(frame_event.get());
                    return;
                }

                auto frame_size = frame.ContentSize();
                auto surface = frame.Surface();
                auto texture = get_texture_from_surface(surface);
                if (!texture) return;

                D3D11_TEXTURE2D_DESC desc{};
                texture->GetDesc(&desc);

                D3D11_TEXTURE2D_DESC staging_desc{};
                staging_desc.Width = desc.Width;
                staging_desc.Height = desc.Height;
                staging_desc.MipLevels = 1;
                staging_desc.ArraySize = 1;
                staging_desc.Format = desc.Format;
                staging_desc.SampleDesc.Count = 1;
                staging_desc.Usage = D3D11_USAGE_STAGING;
                staging_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;

                ComPtr<ID3D11Texture2D> staging;
                ComPtr<ID3D11Device> tex_device;
                texture->GetDevice(&tex_device);
                hr = tex_device->CreateTexture2D(&staging_desc, nullptr, &staging);
                if (FAILED(hr)) return;

                ComPtr<ID3D11DeviceContext> ctx;
                tex_device->GetImmediateContext(&ctx);
                ctx->CopyResource(staging.Get(), texture.Get());

                D3D11_MAPPED_SUBRESOURCE mapped{};
                hr = ctx->Map(staging.Get(), 0, D3D11_MAP_READ, 0, &mapped);
                if (FAILED(hr)) return;

                img.width = static_cast<uint32_t>(frame_size.Width);
                img.height = static_cast<uint32_t>(frame_size.Height);
                img.stride = img.width * 4;

                uint32_t copy_bytes = img.stride;
                uint32_t total = copy_bytes * img.height;
                uint8_t* buf = (uint8_t*)malloc(total);
                if (!buf) return;

                auto* src = static_cast<const uint8_t*>(mapped.pData);
                for (uint32_t y = 0; y < img.height; ++y) {
                    memcpy(buf + y * copy_bytes, src + y * mapped.RowPitch, copy_bytes);
                }

                ctx->Unmap(staging.Get(), 0);

                result.width = img.width;
                result.height = img.height;
                result.stride = img.stride;
                result.data = buf;
                result.data_len = total;
                result.success = true;
                SetEvent(frame_event.get());
            } catch (...) {
                SetEvent(frame_event.get());
            }
        });

        session.StartCapture();
        DWORD wait_result = WaitForSingleObject(frame_event.get(), timeout_ms);
        session.Close();
        frame_pool.Close();

        if (wait_result != WAIT_OBJECT_0 || !result.success) {
            if (result.data) free(result.data);
            result = {};
            result.error_code = 3;
            return result;
        }

        return result;

    } catch (const winrt::hresult_error& ex) {
        if (result.data) free(result.data);
        result = {};
        result.error_code = static_cast<uint32_t>(ex.code());
        return result;
    } catch (...) {
        if (result.data) free(result.data);
        result = {};
        result.error_code = 99;
        return result;
    }
}

void wgc_free_capture(WgcCaptureResult* result) {
    if (result && result->data) {
        free(result->data);
        result->data = nullptr;
        result->data_len = 0;
    }
}

} // extern "C"
