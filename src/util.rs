use std::ffi::c_void;

use anyhow::Context;
use image::codecs::png::PngEncoder;
use image::{DynamicImage, ImageFormat};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::DataExchange::*;
use windows::Win32::System::Memory::*;
use windows::Win32::System::Ole::*;

/// Try to read a bitmap image from the clipboard.
pub unsafe fn clipboard_bitmap() -> anyhow::Result<DynamicImage> {
    // check for image content in the clipboard
    IsClipboardFormatAvailable(CF_DIB.0.into())
        .context("clipboard format CF_DIB is not available")?;

    // get access to the clipboard and prevent other applications fro modifying it
    OpenClipboard(HWND(0)).context("couldn't open clipboard")?;

    // get a handle to the clipboard data as a device-independent bitmap (DIB)
    let handle = match GetClipboardData(CF_DIB.0.into()) {
        Ok(handle) => handle.0 as *mut c_void,
        Err(err) => {
            let _ = CloseClipboard();
            return Err(err).context("couldn't get clipboard data");
        }
    };

    // lock the DIB memory object
    let data_ptr = GlobalLock(HGLOBAL(handle));
    if data_ptr.is_null() {
        let _ = CloseClipboard();
        return Err(anyhow::anyhow!("couldn't lock clipboard data"));
    }

    // get the size of the DIB memory object
    let data_len = GlobalSize(HGLOBAL(handle));
    if data_len < std::mem::size_of::<BITMAPINFO>() {
        let _ = GlobalUnlock(HGLOBAL(data_ptr));
        let _ = CloseClipboard();
        return Err(anyhow::anyhow!("clipboard data is malformed"));
    }

    // view the DIP memory object as a block of bytes
    let data = std::slice::from_raw_parts(data_ptr as *const u8, data_len);

    // read the bitmap from the clipboard, prepending the file header
    let bitmap = {
        let mut buffer = vec![0u8; std::mem::size_of::<BITMAPFILEHEADER>() + data_len];

        // https://stackoverflow.com/a/51060661
        // https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-bitmapfileheader
        let header = BITMAPFILEHEADER {
            // The file type; must be BM.
            bfType: 0x4D42,
            // The size, in bytes, of the bitmap file.
            bfSize: std::mem::size_of::<BITMAPFILEHEADER>() as u32 + data_len as u32,
            // Reserved; must be zero.
            bfReserved1: 0,
            // Reserved; must be zero.
            bfReserved2: 0,
            // The byte offset from the BITMAPFILEHEADER structure to the bitmap bits.
            bfOffBits: std::mem::size_of::<BITMAPFILEHEADER>() as u32
                + std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        };

        // view the BITMAPFILEHEADER object as a block of bytes
        let header_bytes = std::slice::from_raw_parts(
            &header as *const BITMAPFILEHEADER as *const u8,
            std::mem::size_of::<BITMAPFILEHEADER>(),
        );

        // copy the header and the bitmap into the buffer and return the buffer
        buffer[..std::mem::size_of::<BITMAPFILEHEADER>()].copy_from_slice(header_bytes);
        buffer[std::mem::size_of::<BITMAPFILEHEADER>()..].copy_from_slice(data);
        buffer
    };

    // unlock the DIB memory object
    if let Err(err) = GlobalUnlock(HGLOBAL(data_ptr)) {
        let _ = CloseClipboard();
        return Err(err).context("couldn't unlock clipboard data");
    }

    // remove access to the clipboard and allow other applications to modify the clipboard
    CloseClipboard().context("couldn't close clipboard")?;

    image::load_from_memory_with_format(&bitmap, ImageFormat::Bmp)
        .context("couldn't parse clipboard image")
}

/// Write encode an image in the PNG format and write it to disk.
pub unsafe fn save_image_png(image: &DynamicImage) -> anyhow::Result<()> {
    let filename = format!(
        "clip_{}.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    let start = std::time::Instant::now();
    {
        let file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&filename)
            .context("couldn't open file to write")?;

        let mut writer = std::io::BufWriter::new(file);
        let encoder = PngEncoder::new(&mut writer);

        image
            .write_with_encoder(encoder)
            .context("couldn't encode image")?;
    }
    let elapsed = start.elapsed().as_secs_f64();

    println!("wrote {} in {:.1}ms", filename, elapsed * 1e3);
    Ok(())
}
