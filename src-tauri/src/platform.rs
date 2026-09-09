use std::{
    ffi::c_void,
    path::Path,
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::*,
    Security::Cryptography::*,
    System::{DataExchange::*, Memory::*, Registry::*},
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
pub fn protect_key(path: &Path, key: &str) -> Result<(), String> {
    let mut bytes = key.as_bytes().to_vec();
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_mut_ptr(),
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };
    unsafe {
        if CryptProtectData(
            &input,
            null(),
            null(),
            null_mut(),
            null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        ) == 0
        {
            return Err(
                "Windows could not encrypt your API key. Please try saving it again.".into(),
            );
        }
        let result = std::fs::write(
            path,
            std::slice::from_raw_parts(output.pbData, output.cbData as usize),
        )
        .map_err(|_| "Could not save the encrypted API key. Please try again.".to_string());
        LocalFree(output.pbData as *mut c_void);
        bytes.fill(0);
        result
    }
}
pub fn read_key(path: &Path) -> Result<String, String> {
    let mut bytes = std::fs::read(path)
        .map_err(|_| "Add an OpenRouter API key in Settings to start dictating.".to_string())?;
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_mut_ptr(),
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };
    unsafe {
        if CryptUnprotectData(
            &input,
            null_mut(),
            null(),
            null_mut(),
            null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        ) == 0
        {
            return Err(
                "This Windows account cannot unlock the saved API key. Add it again in Settings."
                    .into(),
            );
        }
        let slice = std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize);
        let result = String::from_utf8(slice.to_vec()).map_err(|_| {
            "The saved API key could not be read. Add it again in Settings.".to_string()
        });
        slice.fill(0);
        LocalFree(output.pbData as *mut c_void);
        result
    }
}
#[derive(Clone, Copy, Default)]
pub struct Target {
    pub window: isize,
    focus: isize,
    caret_x: i32,
    caret_y: i32,
}
pub fn target() -> Target {
    unsafe {
        let window = GetForegroundWindow();
        let mut info: GUITHREADINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
        GetGUIThreadInfo(GetWindowThreadProcessId(window, null_mut()), &mut info);
        Target {
            window: window as isize,
            focus: info.hwndFocus as isize,
            caret_x: info.rcCaret.left,
            caret_y: info.rcCaret.top,
        }
    }
}
pub fn target_matches(expected: Target) -> bool {
    let now = target();
    expected.window != 0
        && now.window == expected.window
        && now.focus == expected.focus
        && now.caret_x == expected.caret_x
        && now.caret_y == expected.caret_y
}
pub fn modifiers_down() -> bool {
    unsafe {
        [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN]
            .iter()
            .any(|v| GetAsyncKeyState(*v as i32) < 0)
    }
}
pub fn escape_down() -> bool {
    unsafe { GetAsyncKeyState(VK_ESCAPE as i32) < 0 }
}
pub fn overlay_no_activate(hwnd: HWND) {
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            style | WS_EX_NOACTIVATE as isize | WS_EX_TOOLWINDOW as isize,
        );
    }
}
pub fn insert_text(text: &str, expected: Target) -> Result<(), String> {
    if !target_matches(expected) {
        return Err("The cursor position changed. Open Bol to copy your transcript.".into());
    }
    if modifiers_down() {
        return Err(
            "Release Ctrl, Alt, Shift, and the Windows key, then copy your transcript from Bol."
                .into(),
        );
    }
    // Unicode input leaves every clipboard format intact and never sends VK_RETURN.
    let mut input = Vec::new();
    for unit in text.encode_utf16() {
        for flag in [KEYEVENTF_UNICODE, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP] {
            input.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: 0,
                        wScan: unit,
                        dwFlags: flag,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            });
        }
    }
    if input.is_empty() {
        return Ok(());
    }
    unsafe {
        if SendInput(
            input.len() as u32,
            input.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        ) != input.len() as u32
        {
            return Err("Windows blocked some or all text input. Check the destination before copying to avoid duplicates.".into());
        }
    }
    Ok(())
}
pub fn copy_text(text: &str) -> Result<(), String> {
    let utf16 = wide(text);
    unsafe {
        if OpenClipboard(null_mut()) == 0 {
            return Err("Clipboard is busy. Try Copy again.".into());
        }
        let memory = GlobalAlloc(GMEM_MOVEABLE, utf16.len() * 2);
        if memory.is_null() {
            CloseClipboard();
            return Err("Could not prepare text for copying. Please try Copy again.".into());
        }
        let data = GlobalLock(memory) as *mut u16;
        if data.is_null() {
            GlobalFree(memory);
            CloseClipboard();
            return Err("Could not prepare text for copying. Please try Copy again.".into());
        }
        std::ptr::copy_nonoverlapping(utf16.as_ptr(), data, utf16.len());
        GlobalUnlock(memory);
        if EmptyClipboard() == 0 || SetClipboardData(13, memory).is_null() {
            GlobalFree(memory);
            CloseClipboard();
            return Err("Could not copy the text. Please try Copy again.".into());
        }
        CloseClipboard();
    }
    Ok(())
}
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    let executable = if enabled {
        Some(
            std::env::current_exe()
                .map_err(|_| "Could not find the Bol app file. Reopen Bol and try again.")?,
        )
    } else {
        None
    };
    unsafe {
        let mut key = null_mut();
        if RegCreateKeyExW(
            HKEY_CURRENT_USER,
            wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run").as_ptr(),
            0,
            null(),
            0,
            KEY_SET_VALUE,
            null(),
            &mut key,
            null_mut(),
        ) != 0
        {
            return Err("Could not change startup settings. Please try again.".into());
        }
        let name = wide("Bol");
        let status = if enabled {
            let exe = executable.as_ref().unwrap();
            let value = wide(&format!("\"{}\" --background", exe.display()));
            RegSetValueExW(
                key,
                name.as_ptr(),
                0,
                REG_SZ,
                value.as_ptr() as *const u8,
                (value.len() * 2) as u32,
            )
        } else {
            RegDeleteValueW(key, name.as_ptr())
        };
        RegCloseKey(key);
        if status != 0 && !(status == 2 && !enabled) {
            return Err("Could not change startup settings. Please try again.".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn api_key_round_trip_is_windows_protected() {
        let directory =
            std::env::temp_dir().join(format!("bol-credential-test-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("key");
        let key = "synthetic-test-key-not-a-real-credential";
        protect_key(&path, key).unwrap();
        assert_eq!(read_key(&path).unwrap(), key);
        let data = std::fs::read(&path).unwrap();
        assert!(!data.windows(key.len()).any(|w| w == key.as_bytes()));
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(&directory).unwrap();
    }
}
