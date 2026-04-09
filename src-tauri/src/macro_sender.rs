//! Platform-specific macro sender: paste a message into the focused app,
//! then press Enter.
//!
//! - macOS: put the text on the clipboard via `pbcopy`, then Cmd+V via AppleScript.
//! - Windows: type each char as Unicode via `SendInput`.
//!
//! Going through the clipboard / Unicode paths avoids the IME and modifier
//! quirks that AppleScript `keystroke` runs into.

#[cfg(target_os = "macos")]
pub fn send_macro(text: &str) -> Result<(), String> {
    use std::io::Write;

    // 1. Put text on the clipboard via pbcopy
    let mut child = std::process::Command::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("pbcopy spawn failed: {e}"))?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("pbcopy write failed: {e}"))?;
    }
    child
        .wait()
        .map_err(|e| format!("pbcopy wait failed: {e}"))?;

    // 2. Cmd+V to paste, then Return (give the paste time to actually land).
    //
    // Use `key code 9` (the V key) instead of `keystroke "v"`. The `keystroke`
    // form goes through character-to-key translation and occasionally emits
    // the V with the wrong modifier (showing up as Option+V / ^[v in terminals).
    // `key code` sends the raw virtual-key event so the Command modifier sticks.
    let script = r#"tell application "System Events"
  key code 9 using {command down}
  delay 0.2
  key code 36
  delay 0.05
end tell"#;
    std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|e| format!("osascript failed: {e}"))?;
    Ok(())
}

#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_MENU, VK_RETURN, VK_TAB,
};

#[cfg(target_os = "windows")]
pub fn send_macro(text: &str) -> Result<(), String> {
    unsafe {
        // Type the message as Unicode (handles CJK / emoji without IME issues)
        for ch in text.chars() {
            send_char(ch);
        }

        // Enter
        send_key_press(VK_RETURN);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
unsafe fn send_key_combo(keys: &[VIRTUAL_KEY]) {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;

    let mut inputs: Vec<INPUT> = Vec::new();

    // Key downs
    for &vk in keys {
        inputs.push(make_key_input(vk, KEYBD_EVENT_FLAGS(0)));
    }
    // Key ups (reverse order)
    for &vk in keys.iter().rev() {
        inputs.push(make_key_input(vk, KEYEVENTF_KEYUP));
    }

    SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
}

#[cfg(target_os = "windows")]
unsafe fn send_key_press(vk: VIRTUAL_KEY) {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    let inputs = [
        make_key_input(vk, KEYBD_EVENT_FLAGS(0)),
        make_key_input(vk, KEYEVENTF_KEYUP),
    ];
    SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
}

#[cfg(target_os = "windows")]
unsafe fn send_char(ch: char) {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: ch as u16,
                    dwFlags: KEYEVENTF_UNICODE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: ch as u16,
                    dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
}

#[cfg(target_os = "windows")]
fn make_key_input(
    vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY,
    flags: windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS,
) -> windows::Win32::UI::Input::KeyboardAndMouse::INPUT {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(target_os = "windows")]
pub fn alt_tab() {
    unsafe {
        send_key_combo(&[
            windows::Win32::UI::Input::KeyboardAndMouse::VK_MENU,
            windows::Win32::UI::Input::KeyboardAndMouse::VK_TAB,
        ]);
    }
}

// Linux: not yet implemented, but stub to compile
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn send_macro(text: &str) -> Result<(), String> {
    eprintln!("[guaiguai-claude] keyboard injection not implemented for this platform. would send: {text}");
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn alt_tab() {}
