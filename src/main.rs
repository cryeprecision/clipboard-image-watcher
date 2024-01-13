mod util;
use util::*;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::*;
use windows_sys::Win32::UI::WindowsAndMessaging as Ui;

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match msg {
        Ui::WM_CREATE => {
            if DataExchange::AddClipboardFormatListener(hwnd) == 0 {
                eprintln!("couldn't add clipboard listener ({})", GetLastError());
                std::process::exit(1);
            }
        }
        Ui::WM_DESTROY => {
            if DataExchange::RemoveClipboardFormatListener(hwnd) == 0 {
                eprintln!("couldn't remove clipboard listener ({})", GetLastError());
                std::process::exit(1);
            }
        }
        Ui::WM_CLIPBOARDUPDATE => {
            let start = std::time::Instant::now();
            let result = clipboard_save_bitmap();
            let elapsed = start.elapsed().as_secs_f64();
            match result {
                Err(err) => eprintln!("error: {}", err),
                Ok(_) => println!("saved clipboard image ({:.3}ms)", elapsed * 1e3),
            }
        }
        // ignore all other messages
        _ => (),
    }

    Ui::DefWindowProcA(hwnd, msg, w_param, l_param)
}

fn main() {
    // https://stackoverflow.com/a/65857206
    unsafe {
        let mut window = std::mem::zeroed::<Ui::WNDCLASSEXA>();
        window.cbSize = std::mem::size_of::<Ui::WNDCLASSEXA>() as u32;
        window.lpfnWndProc = Some(window_proc);
        window.lpszClassName = windows_sys::s!("ClipboardWatcher");

        if Ui::RegisterClassExA(&window as *const Ui::WNDCLASSEXA) == 0 {
            eprintln!("couldn't register window ({})", GetLastError());
            std::process::exit(1);
        }

        let handle = Ui::CreateWindowExA(
            0,
            windows_sys::s!("ClipboardWatcher"),
            windows_sys::s!(""),
            0,
            0,
            0,
            0,
            0,
            Ui::HWND_MESSAGE,
            0,
            LibraryLoader::GetModuleHandleA(std::ptr::null()),
            std::ptr::null(),
        );
        if handle == 0 {
            eprintln!("couldn't create window ({})", GetLastError());
            std::process::exit(1);
        }

        println!("watching the clipboard...");

        let mut msg = std::mem::zeroed::<Ui::MSG>();
        while Ui::GetMessageA(&mut msg as *mut Ui::MSG, 0, 0, 0) != FALSE {
            Ui::TranslateMessage(&msg as *const Ui::MSG);
            Ui::DispatchMessageA(&msg as *const Ui::MSG);
        }
        eprintln!("error receiving message ({})", GetLastError());
        std::process::exit(1);
    }
}
