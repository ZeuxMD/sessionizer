#[cfg(target_os = "windows")]
mod imp {
    use crate::control::persist_signal;
    use crate::session::{classify_end_session, classify_power_broadcast};
    use tauri::{Runtime, WebviewWindow};
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        UI::{
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::{WM_ENDSESSION, WM_NCDESTROY, WM_POWERBROADCAST},
        },
    };

    const SESSION_SUBCLASS_ID: usize = 0x5345_5353;

    pub fn install<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), String> {
        let hwnd = window.hwnd().map_err(|e| e.to_string())?;
        let installed = unsafe {
            SetWindowSubclass(
                hwnd.0 as HWND,
                Some(session_subclass_proc),
                SESSION_SUBCLASS_ID,
                0,
            )
        };

        if installed != 0 {
            Ok(())
        } else {
            Err("Failed to install Windows session hook".to_string())
        }
    }

    unsafe extern "system" fn session_subclass_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _subclass_id: usize,
        _ref_data: usize,
    ) -> LRESULT {
        match msg {
            WM_ENDSESSION => {
                let _ = persist_signal(classify_end_session(wparam != 0, lparam as usize));
            }
            WM_POWERBROADCAST => {
                let _ = persist_signal(classify_power_broadcast(wparam));
            }
            WM_NCDESTROY => {
                let _ =
                    RemoveWindowSubclass(hwnd, Some(session_subclass_proc), SESSION_SUBCLASS_ID);
            }
            _ => {}
        }

        DefSubclassProc(hwnd, msg, wparam, lparam)
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use tauri::{Runtime, WebviewWindow};

    pub fn install<R: Runtime>(_window: &WebviewWindow<R>) -> Result<(), String> {
        Ok(())
    }
}

pub use imp::install;
