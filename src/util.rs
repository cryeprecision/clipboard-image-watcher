use image::codecs::png::PngEncoder;
use image::{DynamicImage, ImageFormat};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi;
use windows_sys::Win32::System::{DataExchange, Memory, Ole};

const CLIPBOARD_FORMATS: [(u16, &str); 26] = [
    (Ole::CF_BITMAP, "CF_BITMAP"),
    (Ole::CF_DIB, "CF_DIB"),
    (Ole::CF_DIBV5, "CF_DIBV5"),
    (Ole::CF_DIF, "CF_DIF"),
    (Ole::CF_DSPBITMAP, "CF_DSPBITMAP"),
    (Ole::CF_DSPENHMETAFILE, "CF_DSPENHMETAFILE"),
    (Ole::CF_DSPMETAFILEPICT, "CF_DSPMETAFILEPICT"),
    (Ole::CF_DSPTEXT, "CF_DSPTEXT"),
    (Ole::CF_ENHMETAFILE, "CF_ENHMETAFILE"),
    (Ole::CF_GDIOBJFIRST, "CF_GDIOBJFIRST"),
    (Ole::CF_GDIOBJLAST, "CF_GDIOBJLAST"),
    (Ole::CF_HDROP, "CF_HDROP"),
    (Ole::CF_LOCALE, "CF_LOCALE"),
    (Ole::CF_METAFILEPICT, "CF_METAFILEPICT"),
    (Ole::CF_OEMTEXT, "CF_OEMTEXT"),
    (Ole::CF_OWNERDISPLAY, "CF_OWNERDISPLAY"),
    (Ole::CF_PALETTE, "CF_PALETTE"),
    (Ole::CF_PENDATA, "CF_PENDATA"),
    (Ole::CF_PRIVATEFIRST, "CF_PRIVATEFIRST"),
    (Ole::CF_PRIVATELAST, "CF_PRIVATELAST"),
    (Ole::CF_RIFF, "CF_RIFF"),
    (Ole::CF_SYLK, "CF_SYLK"),
    (Ole::CF_TEXT, "CF_TEXT"),
    (Ole::CF_TIFF, "CF_TIFF"),
    (Ole::CF_UNICODETEXT, "CF_UNICODETEXT"),
    (Ole::CF_WAVE, "CF_WAVE"),
];

#[allow(non_snake_case, dead_code)]
pub fn GetPredefinedClipboardFormatName(format_id: u16) -> Option<&'static str> {
    CLIPBOARD_FORMATS
        .iter()
        .find(|&&(id, _)| id == format_id)
        .map(|&(_, name)| name)
}

#[allow(dead_code)]
pub unsafe fn clipboard_available_formats() -> Vec<&'static str> {
    CLIPBOARD_FORMATS
        .iter()
        .filter_map(|&(format, name)| {
            (DataExchange::IsClipboardFormatAvailable(format as u32) != FALSE).then_some(name)
        })
        .collect()
}

#[allow(dead_code)]
pub unsafe fn clipboard_formats() -> Vec<String> {
    let mut str_buf = [0u8; 80]; // just pray that this is enough
    let mut ret_buf = Vec::new();

    if DataExchange::OpenClipboard(0) == FALSE {
        return ret_buf;
    }

    let mut format_idx = DataExchange::EnumClipboardFormats(0);
    while format_idx != 0 {
        let format_len = DataExchange::GetClipboardFormatNameA(
            format_idx,
            str_buf.as_mut_ptr(),
            str_buf.len() as i32,
        ) as usize;

        if format_len != 0 {
            let format_name = std::str::from_utf8(&str_buf[..format_len]).unwrap();
            ret_buf.push(format_name.to_string());
        } else if let Some(format_name) = GetPredefinedClipboardFormatName(format_idx as u16) {
            ret_buf.push(format_name.to_string());
        } else {
            ret_buf.push("[unknown format]".to_string());
        }

        format_idx = DataExchange::EnumClipboardFormats(format_idx);
    }

    let _ = DataExchange::CloseClipboard();
    ret_buf
}

#[allow(dead_code)]
pub unsafe fn clipboard_text() -> Result<String, &'static str> {
    if DataExchange::IsClipboardFormatAvailable(Ole::CF_TEXT as u32) == FALSE {
        return Err("wanted clipboard format is not available");
    }

    if DataExchange::OpenClipboard(0) == FALSE {
        return Err("couldn't open clipboard");
    }

    let data = DataExchange::GetClipboardData(Ole::CF_TEXT as u32);
    if data == 0 {
        DataExchange::CloseClipboard();
        return Err("couldn't get clipboard data");
    }

    let locked_data = Memory::GlobalLock(data as *mut std::ffi::c_void);
    if locked_data.is_null() {
        DataExchange::CloseClipboard();
        return Err("couldn't lock clipboard data");
    }

    let text = std::ffi::CStr::from_ptr(locked_data as *mut i8);

    if Memory::GlobalUnlock(locked_data) == FALSE && GetLastError() != NO_ERROR {
        DataExchange::CloseClipboard();
        return Err("couldn't unlock clipboard data");
    }

    if DataExchange::CloseClipboard() == FALSE {
        return Err("couldn't close clipboard");
    }

    Ok(text.to_str().unwrap().to_string())
}

pub unsafe fn clipboard_bitmap() -> Result<DynamicImage, &'static str> {
    if DataExchange::IsClipboardFormatAvailable(Ole::CF_DIB as u32) == FALSE {
        return Err("wanted clipboard format is not available");
    }

    if DataExchange::OpenClipboard(0) == FALSE {
        return Err("couldn't open clipboard");
    }

    let handle = DataExchange::GetClipboardData(Ole::CF_DIB as u32);
    if handle == 0 {
        DataExchange::CloseClipboard();
        return Err("couldn't get clipboard data");
    }

    let data_ptr = Memory::GlobalLock(handle as *mut std::ffi::c_void);
    if data_ptr.is_null() {
        DataExchange::CloseClipboard();
        return Err("couldn't lock clipboard data");
    }

    let data_len = Memory::GlobalSize(handle as *mut std::ffi::c_void);
    if data_len < std::mem::size_of::<Gdi::BITMAPINFO>() {
        Memory::GlobalUnlock(data_ptr);
        DataExchange::CloseClipboard();
        return Err("clipboard data is malformed");
    }

    // read the bitmap from the clipboard, appending a missing header
    let bitmap = {
        let mut buffer = Vec::new();

        // https://stackoverflow.com/a/51060661
        // https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-bitmapfileheader
        let header = Gdi::BITMAPFILEHEADER {
            // The file type; must be BM.
            bfType: 0x4D42,
            // The size, in bytes, of the bitmap file.
            bfSize: std::mem::size_of::<Gdi::BITMAPFILEHEADER>() as u32 + data_len as u32,
            // Reserved; must be zero.
            bfReserved1: 0,
            // Reserved; must be zero.
            bfReserved2: 0,
            // The offset, in bytes, from the beginning of the BITMAPFILEHEADER structure
            // to the bitmap bits.
            bfOffBits: std::mem::size_of::<Gdi::BITMAPFILEHEADER>() as u32
                + std::mem::size_of::<Gdi::BITMAPINFOHEADER>() as u32,
        };

        buffer.extend_from_slice(std::slice::from_raw_parts(
            &header as *const Gdi::BITMAPFILEHEADER as *const u8,
            std::mem::size_of::<Gdi::BITMAPFILEHEADER>(),
        ));
        buffer.extend_from_slice(std::slice::from_raw_parts(data_ptr as *const u8, data_len));

        buffer
    };

    if Memory::GlobalUnlock(data_ptr) == FALSE && GetLastError() != NO_ERROR {
        DataExchange::CloseClipboard();
        return Err("couldn't unlock clipboard data");
    }

    if DataExchange::CloseClipboard() == FALSE {
        return Err("couldn't close clipboard");
    }

    image::load_from_memory_with_format(&bitmap, ImageFormat::Bmp)
        .map_err(|_| "couldn't parse clipboard image")
}

pub unsafe fn clipboard_save_bitmap() -> Result<(), &'static str> {
    let image = clipboard_bitmap()?;

    let filename = format!(
        "clip_{}.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(filename)
        .map_err(|_| "couldn't open file to write")?;

    let encoder = PngEncoder::new(&mut file);
    image
        .write_with_encoder(encoder)
        .map_err(|_| "couldn't encode image")
}
