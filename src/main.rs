mod util;

use anyhow::Context;
use util::*;
//
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::DataExchange::*;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::UI::WindowsAndMessaging::*;

const CLASS_NAME: PCSTR = s!("ClipboardWatcher");

unsafe fn on_clipboard_update() {
    let Ok(bitmap) = clipboard_bitmap() else {
        return;
    };
    if let Err(err) = clipboard_save_image(&bitmap) {
        eprintln!("couldn't save clipboard image: {:?}", err);
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match msg {
        // https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-create
        WM_CREATE => AddClipboardFormatListener(hwnd)
            .context("couldn't add clipboard format listener")
            .unwrap(),
        // https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-destroy
        WM_DESTROY => RemoveClipboardFormatListener(hwnd)
            .context("couldn't remove clipboard format listener")
            .unwrap(),
        // https://learn.microsoft.com/en-us/windows/win32/dataxchg/wm-clipboardupdate
        WM_CLIPBOARDUPDATE => on_clipboard_update(),
        // ignore all other messages
        _ => (),
    }
    DefWindowProcA(hwnd, msg, w_param, l_param)
}

fn main() {
    // https://stackoverflow.com/a/65857206
    unsafe {
        let mut window = std::mem::zeroed::<WNDCLASSEXA>();
        window.cbSize = std::mem::size_of::<WNDCLASSEXA>() as u32;
        window.lpfnWndProc = Some(window_proc);
        window.lpszClassName = CLASS_NAME;

        if RegisterClassExA(&window as *const WNDCLASSEXA) == 0 {
            eprintln!("couldn't register window ({:?})", GetLastError());
            std::process::exit(1);
        }

        let module = GetModuleHandleA(PCSTR(std::ptr::null()))
            .context("couldn't get handle to own module")
            .unwrap();

        let handle = CreateWindowExA(
            WINDOW_EX_STYLE(0),
            CLASS_NAME,
            PCSTR(std::ptr::null()),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            HMENU(0),
            module,
            None,
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
