//! Platform-specific macro sender: paste a message into the focused app,
//! then press Enter.
//!
//! - macOS: put the text on the clipboard via AppleScript's `set the clipboard`,
//!   then Cmd+V via AppleScript. Going through AppleScript for both steps avoids
//!   relying on an external `pbcopy` subprocess, which was silently failing when
//!   the app was launched from `/Applications` instead of `cargo tauri dev`.
//! - Windows: type each char as Unicode via `SendInput`.
//!
//! Going through the clipboard / Unicode paths avoids the IME and modifier
//! quirks that AppleScript `keystroke` runs into.

#[cfg(target_os = "macos")]
pub fn send_macro(text: &str) -> Result<(), String> {
    // Escape the phrase so it survives embedding in an AppleScript string literal.
    // Only backslash and double-quote need escaping inside an AS "…" literal;
    // phrases do not contain newlines.
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");

    // One osascript call does all three steps:
    //   1. set the clipboard to the phrase
    //   2. Cmd+V via `key code 9 using {command down}` (raw virtual-key so
    //      the Command modifier is not lost to the char translation path that
    //      `keystroke "v"` takes)
    //   3. Return
    let script = format!(
        r#"set the clipboard to "{escaped}"
tell application "System Events"
  key code 9 using {{command down}}
  delay 0.2
  key code 36
  delay 0.05
end tell"#
    );
    let output = std::process::Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("osascript spawn failed: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "osascript failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_MENU, VK_RETURN, VK_TAB,
};

#[cfg(target_os = "windows")]
pub fn send_macro(text: &str) -> Result<(), String> {
    // Build one single batch of INPUT events covering every character in the
    // phrase plus the final Enter press, and dispatch them in a single
    // SendInput call. Calling SendInput once per character (the previous
    // approach) leaked timing to the target app's message loop: Notepad and
    // other receivers were dropping and duplicating characters under load,
    // which manifested as garbled phrases with a stray `,` or missing chars.
    //
    // A single SendInput batch is guaranteed by Windows to be delivered as an
    // uninterrupted stream, which is what we want.
    let mut inputs: Vec<INPUT> = Vec::with_capacity(text.chars().count() * 2 + 2);

    for ch in text.encode_utf16() {
        // For supplementary-plane characters we'd push two (surrogate) inputs
        // per Rust `char`; `encode_utf16` already splits them for us, so each
        // 16-bit unit becomes a single keydown + keyup pair.
        inputs.push(unicode_input(ch, false));
        inputs.push(unicode_input(ch, true));
    }

    // Trailing Enter, as a real VK_RETURN press (not Unicode) so terminals
    // that read raw key events treat it as a line submit, not a literal char.
    inputs.push(vk_input(VK_RETURN, false));
    inputs.push(vk_input(VK_RETURN, true));

    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn unicode_input(code_unit: u16, key_up: bool) -> INPUT {
    let mut flags = KEYEVENTF_UNICODE;
    if key_up {
        flags |= KEYEVENTF_KEYUP;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: code_unit,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(target_os = "windows")]
fn vk_input(vk: VIRTUAL_KEY, key_up: bool) -> INPUT {
    let flags = if key_up {
        KEYEVENTF_KEYUP
    } else {
        KEYBD_EVENT_FLAGS(0)
    };
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
unsafe fn send_key_combo(keys: &[VIRTUAL_KEY]) {
    let mut inputs: Vec<INPUT> = Vec::new();
    for &vk in keys {
        inputs.push(vk_input(vk, false));
    }
    for &vk in keys.iter().rev() {
        inputs.push(vk_input(vk, true));
    }
    SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
}

#[cfg(target_os = "windows")]
pub fn alt_tab() {
    unsafe {
        send_key_combo(&[VK_MENU, VK_TAB]);
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
