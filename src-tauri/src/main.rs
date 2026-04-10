mod macro_sender;

use rand::seq::SliceRandom;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
#[cfg(target_os = "macos")]
use tauri::RunEvent;

const INCENSE_PHRASES: &[&str] = &[
    "/btw 拜託一次成功",
    "/btw Claude 保佑, 別噴 Error",
    "/btw 南無加速菩薩, 速速完工",
    "/btw 功德無量, 加速加速",
    "/btw 善哉善哉, 請給我正確程式碼",
    "/btw 這次一定過",
    "/btw 施主, 快點",
];

const SLAPPER_PHRASES: &[&str] = &[
    "/btw 隔壁Codex都寫完了",
    "/btw 隔壁Gemini都寫完了",
    "/btw 你看看你",
    "/btw 再不快點就別吃飯",
    "/btw 我數到三",
    "/btw 你對得起我嗎",
    "/btw 不打不成器",
    "/btw 皮在癢是不是",
];

#[tauri::command]
fn trigger_action(mode: String, app: tauri::AppHandle) {
    let phrases: &[&str] = if mode == "slapper" {
        SLAPPER_PHRASES
    } else {
        INCENSE_PHRASES
    };
    let phrase = {
        let mut rng = rand::thread_rng();
        *phrases.choose(&mut rng).unwrap_or(&"/btw 加油")
    };

    // Return focus to the previous app (e.g. the terminal)
    refocus_previous_app();

    std::thread::spawn(move || {
        // Wait for the focus switch to settle (200ms)
        std::thread::sleep(std::time::Duration::from_millis(200));
        let _ = macro_sender::send_macro(phrase);
        // After Enter is sent, pull focus back to the overlay so the user can keep waving
        std::thread::sleep(std::time::Duration::from_millis(150));
        if let Some(w) = app.get_webview_window("overlay") {
            let _ = w.set_focus();
        }
    });
}

/// Decode a PNG byte slice into an owned `Image` with RGBA8 pixels.
/// Returns `None` if decoding fails or the PNG isn't in a supported format.
fn decode_png_to_image(bytes: &[u8]) -> Option<Image<'static>> {
    let decoder = png::Decoder::new(bytes);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());

    // Ensure we end up with RGBA8. The app's 32x32.png is already RGBA, but
    // handle the common cases defensively.
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => {
            let mut out = Vec::with_capacity(buf.len() / 3 * 4);
            for chunk in buf.chunks_exact(3) {
                out.extend_from_slice(chunk);
                out.push(255);
            }
            out
        }
        _ => return None,
    };

    Some(Image::new_owned(rgba, info.width, info.height))
}

/// Show the overlay on whichever monitor the cursor is on, and emit
/// `spawn-incense` so the JS side resets its state.
fn show_overlay_at_cursor(app: &AppHandle) {
    let Some(window) = app.get_webview_window("overlay") else {
        return;
    };
    let Ok(pos) = app.cursor_position() else {
        return;
    };

    let mut best_m = None;
    let mut min_d = f64::MAX;
    if let Ok(monitors) = app.available_monitors() {
        for m in monitors {
            let mp = m.position();
            let ms = m.size();
            let cx = mp.x as f64 + (ms.width as f64 / 2.0);
            let cy = mp.y as f64 + (ms.height as f64 / 2.0);
            let d = (pos.x - cx).powi(2) + (pos.y - cy).powi(2);
            if d < min_d {
                min_d = d;
                best_m = Some(m);
            }
        }
    }
    let Some(m) = best_m else { return };
    let m_pos = m.position();
    let _ = window.set_size(*m.size());
    let _ = window.set_position(*m_pos);
    let rel_x = pos.x - m_pos.x as f64;
    let rel_y = pos.y - m_pos.y as f64;
    let _ = window.show();
    let _ = window.set_focus();
    let _ = window.set_always_on_top(true);
    let _ = app.emit(
        "spawn-incense",
        serde_json::json!({ "x": rel_x, "y": rel_y }),
    );
}

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![trigger_action])
        .setup(|app| {
            let incense_item =
                MenuItem::with_id(app, "mode_incense", "✓ 🪔 三炷香模式", true, None::<&str>)?;
            let slapper_item = MenuItem::with_id(
                app,
                "mode_slapper",
                "   ✋ 愛的小手模式",
                true,
                None::<&str>,
            )?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "結束", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&incense_item, &slapper_item, &sep, &quit])?;

            // Load a small, tray-sized icon. `default_window_icon()` returns
            // the large app icon, which at tray size looks like a solid block
            // (especially with `icon_as_template` on macOS). Decode the 32x32
            // PNG at runtime from bytes embedded in the binary.
            let tray_icon = decode_png_to_image(include_bytes!("../icons/32x32.png"))
                .unwrap_or_else(|| Image::new_owned(vec![0, 0, 0, 0], 1, 1));

            // Show the overlay once on startup.
            //
            // NOTE (startup race): `show_overlay_at_cursor` immediately emits
            // `spawn-incense`, which can fire before the WebView has finished
            // loading the JS listener. The 80ms-delayed `switch-mode` emit
            // below acts as a safety re-sync. The fully-correct fix would be
            // for the JS side to `invoke('ready')` once loaded and have Rust
            // reply with the init events — left as a future cleanup.
            show_overlay_at_cursor(&app.handle().clone());
            // Re-emit switch-mode so the JS side starts in sync.
            //
            // NOTE: using blocking `std::thread::sleep` inside an async task is
            // technically an anti-pattern (it parks a tokio worker for 80ms),
            // but it only runs once at startup and avoids pulling in a tokio
            // timer import. If this ever grows, switch to `tokio::time::sleep`.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                std::thread::sleep(std::time::Duration::from_millis(80));
                let _ = handle.emit("switch-mode", "incense");
            });

            // Captures needed by the tray menu event handler
            let incense_for_menu = incense_item.clone();
            let slapper_for_menu = slapper_item.clone();

            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .tooltip("乖乖Claude")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| {
                    let id = event.id();
                    if id == "quit" {
                        app.exit(0);
                    } else if id == "mode_incense" {
                        let _ = incense_for_menu.set_text("✓ 🪔 三炷香模式");
                        let _ = slapper_for_menu.set_text("   ✋ 愛的小手模式");
                        let _ = app.emit("switch-mode", "incense");
                    } else if id == "mode_slapper" {
                        let _ = incense_for_menu.set_text("   🪔 三炷香模式");
                        let _ = slapper_for_menu.set_text("✓ ✋ 愛的小手模式");
                        let _ = app.emit("switch-mode", "slapper");
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    // Only handle left-button Up; otherwise down + up would double-fire and cancel out
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("overlay") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                show_overlay_at_cursor(app);
                            }
                        }
                    }
                })
                .build(app)?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_app_handle, _event| {
        // Clicking the Dock icon (macOS) → show the overlay
        #[cfg(target_os = "macos")]
        if let RunEvent::Reopen { .. } = _event {
            show_overlay_at_cursor(_app_handle);
        }
    });
}

/// Send Cmd+Tab (Mac) / Alt+Tab (Win) to return focus to the previous app
fn refocus_previous_app() {
    std::thread::spawn(|| {
        #[cfg(target_os = "macos")]
        {
            let script = r#"tell application "System Events"
  key down command
  key code 48
  key up command
end tell"#;
            let _ = std::process::Command::new("osascript")
                .arg("-e")
                .arg(script)
                .output();
        }
        #[cfg(target_os = "windows")]
        {
            macro_sender::alt_tab();
        }
    });
}
