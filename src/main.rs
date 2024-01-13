mod util;

use anyhow::Context;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::DataExchange::*;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::util::*;

const CLASS_NAME: PCSTR = s!("ClipboardWatcher");

/// Invoked every time the clipboard content changes
unsafe fn on_clipboard_update() {
    let Ok(bitmap) = clipboard_bitmap() else {
        return;
    };
    if let Err(err) = save_image_png(&bitmap) {
        eprintln!("couldn't save clipboard image: {:?}", err);
    }
}

/// <https://learn.microsoft.com/en-us/windows/win32/learnwin32/writing-the-window-procedure>
#[allow(non_snake_case)]
unsafe extern "system" fn WindowProc(
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match msg {
        // https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-create
        WM_CREATE => {
            AddClipboardFormatListener(hwnd)
                .context("couldn't add clipboard format listener")
                .unwrap();
        }
        // https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-destroy
        WM_DESTROY => {
            RemoveClipboardFormatListener(hwnd)
                .context("couldn't remove clipboard format listener")
                .unwrap();
        }
        // https://learn.microsoft.com/en-us/windows/win32/dataxchg/wm-clipboardupdate
        WM_CLIPBOARDUPDATE => {
            on_clipboard_update();
        }
        // ignore all other messages
        _ => (),
    }

    // https://learn.microsoft.com/en-us/windows/win32/learnwin32/writing-the-window-procedure#default-message-handling
    DefWindowProcA(hwnd, msg, w_param, l_param)
}

fn main() {
    // based on https://stackoverflow.com/a/65857206
    unsafe {
        let mut window = std::mem::zeroed::<WNDCLASSEXA>();
        window.cbSize = std::mem::size_of::<WNDCLASSEXA>() as u32;
        window.lpfnWndProc = Some(WindowProc);
        window.lpszClassName = CLASS_NAME;

        if RegisterClassExA(&window as *const WNDCLASSEXA) == 0 {
            eprintln!("couldn't register window ({:?})", GetLastError());
            std::process::exit(1);
        }

        let module = GetModuleHandleA(PCSTR(std::ptr::null()))
            .context("couldn't get handle to own module")
            .unwrap();

        // https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-createwindowexa
        let handle = CreateWindowExA(
            WINDOW_EX_STYLE(0),      // Optional window styles.
            CLASS_NAME,              // Window class
            PCSTR(std::ptr::null()), // Window text
            WINDOW_STYLE(0),         // Window style
            CW_USEDEFAULT,           // Size and position
            CW_USEDEFAULT,           // Size and position
            CW_USEDEFAULT,           // Size and position
            CW_USEDEFAULT,           // Size and position
            HWND_MESSAGE,            // Create a message-only window
            HMENU(0),                // Menu
            module,                  // Instance handle
            None,                    // Additional application data
        );

        if handle == HWND(0) {
            eprintln!("couldn't create window ({:?})", GetLastError());
            std::process::exit(1);
        }

        println!("watching the clipboard...");

        let mut msg = std::mem::zeroed::<MSG>();
        while GetMessageA(&mut msg as *mut MSG, HWND(0), 0, 0) != FALSE {
            let _ = TranslateMessage(&msg as *const MSG);
            let _ = DispatchMessageA(&msg as *const MSG);
        }

        eprintln!("error receiving message ({:?})", GetLastError());
        std::process::exit(1);
    }
}
