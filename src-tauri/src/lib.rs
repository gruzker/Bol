mod api;
mod audio;
mod overlay;
mod platform;
mod storage;

use serde::Serialize;
use std::{
    sync::{mpsc, Mutex},
    time::{Duration, Instant},
};
use storage::{HistoryItem, Settings, Storage};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    mic_test: bool,
    phase: String,
    message: String,
    text: String,
    seconds: f64,
    latency_ms: u64,
    cost: Option<f64>,
    session_id: u64,
}
impl Default for Status {
    fn default() -> Self {
        Self {
            mic_test: false,
            phase: "idle".into(),
            message: "Ready to dictate".into(),
            text: String::new(),
            seconds: 0.0,
            latency_ms: 0,
            cost: None,
            session_id: 0,
        }
    }
}
struct Active {
    id: u64,
    stop: Option<mpsc::Sender<bool>>,
    cancellation: CancellationToken,
    target: platform::Target,
    focus_changed: bool,
}
#[derive(Default)]
struct Runtime {
    status: Status,
    active: Option<Active>,
    next_id: u64,
}
impl Runtime {
    fn accepts(&self, id: u64) -> bool {
        self.active
            .as_ref()
            .is_some_and(|a| a.id == id && !a.cancellation.is_cancelled())
    }
    fn cancel_active(&mut self) -> Option<u64> {
        let active = self.active.take()?;
        active.cancellation.cancel();
        if let Some(tx) = active.stop {
            let _ = tx.send(true);
        }
        self.status = Status {
            message: "Cancelled. Nothing was inserted.".into(),
            session_id: active.id,
            ..Status::default()
        };
        Some(active.id)
    }
}
struct AppState {
    storage: Mutex<Storage>,
    runtime: Mutex<Runtime>,
    client: reqwest::Client,
}

fn key(state: &AppState) -> Result<String, String> {
    let path = state
        .storage
        .lock()
        .unwrap()
        .directory
        .join("openrouter.key");
    platform::read_key(&path).or_else(|_| {
        std::env::var("OPENROUTER_API_KEY")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .ok_or("Add an OpenRouter API key in Settings to start dictating.".into())
    })
}
fn publish(app: &AppHandle, status: &Status) {
    let _ = app.emit("dictation-state", status);
}
fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}
fn apply_theme(app: &AppHandle, preference: &str) {
    if let Some(window) = app.get_webview_window("main") {
        let theme = match preference {
            "dark" => Some(tauri::Theme::Dark),
            "light" => Some(tauri::Theme::Light),
            _ => None,
        };
        let _ = window.set_theme(theme);
        let dark = theme.unwrap_or_else(|| window.theme().unwrap_or(tauri::Theme::Light))
            == tauri::Theme::Dark;
        let color = if dark {
            tauri::window::Color(18, 23, 21, 255)
        } else {
            tauri::window::Color(248, 249, 246, 255)
        };
        let _ = window.set_background_color(Some(color));
    }
}
fn show_overlay(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("overlay") {
        let anchor = app
            .state::<AppState>()
            .storage
            .lock()
            .unwrap()
            .settings()
            .icon_position;
        // Reset persisted WebView zoom and let CSS fit the actual client viewport.
        let _ = w.set_zoom(1.0);
        if let Ok(Some(monitor)) = app
            .cursor_position()
            .and_then(|p| app.monitor_from_point(p.x, p.y))
        {
            let scale = monitor.scale_factor();
            // Move to the destination monitor before sizing: WM_DPICHANGED can resize
            // the window when crossing monitors with different scaling.
            let _ = w.set_position(monitor.work_area().position);
            let _ = w.set_size(tauri::PhysicalSize::new(
                (64.0 * scale).round() as u32,
                (64.0 * scale).round() as u32,
            ));
            // Windows may enforce a minimum width larger than the requested size.
            if let Ok(size) = w.outer_size() {
                let _ =
                    w.set_position(overlay::position(monitor.work_area(), size, scale, &anchor));
            }
        }
        let _ = w.show();
    }
}
fn hide_later(app: AppHandle, id: u64) {
    tauri::async_runtime::spawn(async move {
        let delay = {
            let state = app.state::<AppState>();
            let runtime = state.runtime.lock().unwrap();
            if runtime.status.phase == "error" {
                3000
            } else {
                700
            }
        };
        tokio::time::sleep(Duration::from_millis(delay)).await;
        let state = app.state::<AppState>();
        let runtime = state.runtime.lock().unwrap();
        if runtime.active.is_none() && runtime.status.session_id == id {
            if let Some(w) = app.get_webview_window("overlay") {
                let _ = w.hide();
            }
        }
    });
}
fn fail_idle(app: &AppHandle, message: String) {
    let state = app.state::<AppState>();
    let mut runtime = state.runtime.lock().unwrap();
    if runtime.active.is_none() {
        runtime.status.phase = "error".into();
        runtime.status.message = message;
        publish(app, &runtime.status);
        show_overlay(app);
        hide_later(app.clone(), runtime.status.session_id);
    }
}
fn update_phase(app: &AppHandle, id: u64, phase: &str, message: &str) {
    let state = app.state::<AppState>();
    let mut runtime = state.runtime.lock().unwrap();
    if runtime.accepts(id) {
        runtime.status.phase = phase.into();
        runtime.status.message = message.into();
        publish(app, &runtime.status);
    }
}
fn begin(app: &AppHandle, mic_test: bool, from_ui: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    let settings = state.storage.lock().unwrap().settings();
    let api_key = if mic_test {
        String::new()
    } else {
        key(&state)?
    };
    let mut runtime = state.runtime.lock().unwrap();
    if runtime.active.is_some() {
        return Err("A dictation is already in progress.".into());
    }
    runtime.next_id += 1;
    let id = runtime.next_id;
    let cancellation = CancellationToken::new();
    let target = if from_ui || mic_test {
        platform::Target::default()
    } else {
        platform::target()
    };
    let recording = audio::start(app.clone(), settings.microphone.clone(), id);
    runtime.active = Some(Active {
        id,
        stop: Some(recording.stop),
        cancellation: cancellation.clone(),
        target,
        focus_changed: false,
    });
    runtime.status = Status {
        mic_test,
        phase: "recording".into(),
        message: if mic_test {
            "Checking your microphone".into()
        } else {
            "Listening…".into()
        },
        session_id: id,
        ..Status::default()
    };
    publish(app, &runtime.status);
    drop(runtime);
    show_overlay(app);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let work = async {
            let audio = recording
                .result
                .await
                .map_err(|_| "Recording stopped unexpectedly. Please try again.".to_string())??;
            let started = Instant::now();
            if mic_test {
                return Ok((
                    "Microphone detected audio. Test audio was not uploaded.".to_owned(),
                    audio.seconds,
                    None,
                    0,
                ));
            }
            update_phase(&app, id, "transcribing", "Turning speech into text…");
            let state = app.state::<AppState>();
            let (text, cost) =
                api::transcribe(&state.client, &api_key, &audio.wav, &settings).await?;
            drop(audio.wav);
            let text = storage::expand_snippet(&text, &settings.snippets);
            Ok((
                text,
                audio.seconds,
                cost,
                started.elapsed().as_millis() as u64,
            ))
        };
        let result: Result<(String, f64, Option<f64>, u64), String> =
            tokio::select! { _=cancellation.cancelled()=>return, result=work=>result };
        // Wait for the dictation chord to be released before any synthesized input.
        for _ in 0..40 {
            if !platform::modifiers_down() {
                break;
            }
            tokio::select! { _=cancellation.cancelled()=>return, _=tokio::time::sleep(Duration::from_millis(20))=>{} }
        }
        complete(&app, id, result, &settings, mic_test);
    });
    Ok(())
}
fn complete(
    app: &AppHandle,
    id: u64,
    result: Result<(String, f64, Option<f64>, u64), String>,
    settings: &Settings,
    mic_test: bool,
) {
    let state = app.state::<AppState>();
    let mut runtime = state.runtime.lock().unwrap();
    if !runtime.accepts(id) {
        return;
    }
    let active = runtime.active.as_ref().unwrap();
    let target = active.target;
    let moved = active.focus_changed;
    match result {
        Ok((text, seconds, cost, latency)) => {
            let message = if mic_test {
                text.clone()
            } else if settings.auto_insert && target.window != 0 {
                if moved {
                    "The cursor position changed. Open Bol to copy your transcript.".into()
                } else {
                    platform::insert_text(&text, target)
                        .map(|_| "Text sent to your app.".into())
                        .unwrap_or_else(|e| e)
                }
            } else {
                "Your transcript is ready to copy.".into()
            };
            let mut message = message;
            if !mic_test && settings.history_enabled {
                if let Err(e) = state.storage.lock().unwrap().add(
                    &text,
                    &settings.language,
                    seconds,
                    cost,
                    latency,
                ) {
                    message = format!("{message} {e}");
                }
            }
            runtime.status = Status {
                mic_test,
                phase: "done".into(),
                message,
                text: if mic_test { String::new() } else { text },
                seconds,
                cost,
                latency_ms: latency,
                session_id: id,
            };
        }
        Err(e) => {
            runtime.status.phase = "error".into();
            runtime.status.message = e;
        }
    }
    runtime.active = None;
    publish(app, &runtime.status);
    drop(runtime);
    hide_later(app.clone(), id);
}
fn finish(app: &AppHandle) {
    let state = app.state::<AppState>();
    let mut runtime = state.runtime.lock().unwrap();
    if runtime.status.phase != "recording" {
        return;
    }
    if let Some(active) = runtime.active.as_mut() {
        if let Some(tx) = active.stop.take() {
            let _ = tx.send(false);
        }
    }
    runtime.status.phase = "transcribing".into();
    runtime.status.message = "Finishing your recording…".into();
    publish(app, &runtime.status);
}
fn cancel(app: &AppHandle) {
    let state = app.state::<AppState>();
    let mut runtime = state.runtime.lock().unwrap();
    if runtime.cancel_active().is_some() {
        publish(app, &runtime.status);
        if let Some(w) = app.get_webview_window("overlay") {
            let _ = w.hide();
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    settings: Settings,
    status: Status,
    history: Vec<HistoryItem>,
    has_key: bool,
}
#[tauri::command]
fn get_snapshot(state: State<AppState>) -> Result<Snapshot, String> {
    let status = state.runtime.lock().unwrap().status.clone();
    let has_key = key(&state).is_ok();
    let store = state.storage.lock().unwrap();
    Ok(Snapshot {
        settings: store.settings(),
        status,
        history: store.history()?,
        has_key,
    })
}
#[tauri::command]
fn list_microphones() -> Result<Vec<String>, String> {
    audio::devices()
}
fn parse_shortcut(value: &str) -> Result<Shortcut, String> {
    if !(value.contains("Control+") || value.contains("Alt+")) || value.len() > 80 {
        return Err("Include Ctrl or Alt in your shortcut, for example Ctrl+Shift+Space.".into());
    }
    value
        .parse::<Shortcut>()
        .map_err(|_| "That shortcut is not supported. Try Ctrl+Shift+Space.".into())
}
#[tauri::command]
fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    settings.validate()?;
    let shortcut = parse_shortcut(&settings.shortcut)?;
    let state = app.state::<AppState>();
    let runtime = state.runtime.lock().unwrap();
    if runtime.active.is_some() {
        return Err("Finish or cancel dictation before changing settings.".into());
    }
    let store = state.storage.lock().unwrap();
    let old = store.settings();
    if old.shortcut != settings.shortcut {
        app.global_shortcut()
            .register(shortcut)
            .map_err(|_| "That shortcut is already in use. Choose another.")?;
    }
    if old.launch_at_login != settings.launch_at_login {
        if let Err(e) = platform::set_autostart(settings.launch_at_login) {
            if old.shortcut != settings.shortcut {
                let _ = app.global_shortcut().unregister(shortcut);
            }
            return Err(e);
        }
    }
    if let Err(e) = store.save_settings(&settings) {
        if old.shortcut != settings.shortcut {
            let _ = app.global_shortcut().unregister(shortcut);
        }
        if old.launch_at_login != settings.launch_at_login {
            let _ = platform::set_autostart(old.launch_at_login);
        }
        return Err(e);
    }
    if old.shortcut != settings.shortcut {
        if let Ok(old_shortcut) = parse_shortcut(&old.shortcut) {
            let _ = app.global_shortcut().unregister(old_shortcut);
        }
    }
    if old.theme != settings.theme {
        apply_theme(&app, &settings.theme);
    }
    Ok(())
}
#[tauri::command]
async fn save_api_key(app: AppHandle, api_key: String) -> Result<(), String> {
    let api_key = api_key.trim();
    if api_key.len() < 12 || api_key.len() > 512 || api_key.contains(char::is_whitespace) {
        return Err("Enter a valid OpenRouter API key.".into());
    }
    let state = app.state::<AppState>();
    api::test_key(&state.client, api_key).await?;
    let path = state
        .storage
        .lock()
        .unwrap()
        .directory
        .join("openrouter.key");
    platform::protect_key(&path, api_key)
}
#[tauri::command]
async fn test_connection(app: AppHandle) -> Result<String, String> {
    let state = app.state::<AppState>();
    let key = key(&state)?;
    api::test_key(&state.client, &key).await?;
    Ok("API key accepted. Start a dictation to check transcription access.".into())
}
#[tauri::command]
fn remove_api_key(state: State<AppState>) -> Result<(), String> {
    let path = state
        .storage
        .lock()
        .unwrap()
        .directory
        .join("openrouter.key");
    if path.exists() {
        std::fs::remove_file(path)
            .map_err(|_| "Could not remove the saved key. Please try again.")?;
    }
    Ok(())
}
#[tauri::command]
fn start_recording(app: AppHandle, mic_test: bool) -> Result<(), String> {
    begin(&app, mic_test, true)
}
#[tauri::command]
fn stop_recording(app: AppHandle) {
    finish(&app);
}
#[tauri::command]
fn cancel_recording(app: AppHandle) {
    cancel(&app);
}
#[tauri::command]
fn copy_text(text: String) -> Result<(), String> {
    platform::copy_text(&text)
}
#[tauri::command]
fn delete_history(state: State<AppState>, id: Option<i64>) -> Result<(), String> {
    state.storage.lock().unwrap().delete(id)
}
#[tauri::command]
fn hide_main(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}
#[tauri::command]
fn quit_app(app: AppHandle) {
    cancel(&app);
    app.exit(0);
}
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            show_main(app)
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    let state = app.state::<AppState>();
                    let settings = state.storage.lock().unwrap().settings();
                    if parse_shortcut(&settings.shortcut).ok().as_ref() != Some(shortcut) {
                        return;
                    }
                    let phase = state.runtime.lock().unwrap().status.phase.clone();
                    if event.state() == ShortcutState::Pressed {
                        if phase == "recording" && settings.activation == "toggle" {
                            finish(app);
                        } else if !["recording", "transcribing"].contains(&phase.as_str()) {
                            if let Err(e) = begin(app, false, false) {
                                fail_idle(app, e);
                            }
                        }
                    } else if settings.activation == "hold" {
                        finish(app);
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            list_microphones,
            save_settings,
            save_api_key,
            test_connection,
            remove_api_key,
            start_recording,
            stop_recording,
            cancel_recording,
            copy_text,
            delete_history,
            hide_main,
            quit_app
        ])
        .setup(|app| {
            let directory = app.path().app_local_data_dir()?;
            let storage = Storage::open(directory).map_err(std::io::Error::other)?;
            let settings = storage.settings();
            apply_theme(app.handle(), &settings.theme);
            app.manage(AppState {
                storage: Mutex::new(storage),
                runtime: Mutex::new(Runtime::default()),
                client: api::client(),
            });
            if let Err(e) = parse_shortcut(&settings.shortcut).and_then(|s| {
                app.global_shortcut()
                    .register(s)
                    .map_err(|_| "Your shortcut is unavailable. Choose another in Settings.".into())
            }) {
                fail_idle(app.handle(), e);
            }
            if let Some(w) = app.get_webview_window("overlay") {
                platform::overlay_no_activate(w.hwnd()?.0 as _);
                let _ = w.set_ignore_cursor_events(true);
            }
            let show = MenuItem::with_id(app, "show", "Open Bol", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Bol", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Bol — speech to text")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main(app),
                    "quit" => {
                        cancel(app);
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;
            if std::env::args().any(|a| a == "--background") {
                if let Some(w) = app.get_webview_window("main") {
                    w.hide()?;
                }
            }
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut escaped = false;
                loop {
                    std::thread::sleep(Duration::from_millis(25));
                    let down = platform::escape_down();
                    if down && !escaped {
                        cancel(&handle);
                    }
                    escaped = down;
                    let state = handle.state::<AppState>();
                    let mut runtime = state.runtime.lock().unwrap();
                    if let Some(active) = runtime.active.as_mut() {
                        if active.target.window != 0 && !platform::target_matches(active.target) {
                            active.focus_changed = true;
                        }
                    }
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("Bol could not start");
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    fn active(id: u64, stop: Option<mpsc::Sender<bool>>) -> Active {
        Active {
            id,
            stop,
            cancellation: CancellationToken::new(),
            target: platform::Target::default(),
            focus_changed: false,
        }
    }
    #[test]
    fn cancellation_stops_audio_and_rejects_late_results() {
        let (tx, rx) = mpsc::channel();
        let mut runtime = Runtime {
            active: Some(active(1, Some(tx))),
            ..Runtime::default()
        };
        let token = runtime.active.as_ref().unwrap().cancellation.clone();
        assert!(runtime.accepts(1));
        assert_eq!(runtime.cancel_active(), Some(1));
        assert!(rx.recv().unwrap());
        assert!(token.is_cancelled());
        assert!(!runtime.accepts(1));
        assert_eq!(runtime.status.text, "");
        runtime.active = Some(active(2, None));
        assert!(!runtime.accepts(1));
        assert!(runtime.accepts(2));
    }
    #[test]
    fn completed_session_cannot_insert_twice() {
        let mut runtime = Runtime {
            active: Some(active(7, None)),
            ..Runtime::default()
        };
        assert!(runtime.accepts(7));
        runtime.active = None;
        assert!(!runtime.accepts(7));
    }
    #[test]
    fn cancellation_during_transcription_clears_text() {
        let mut runtime = Runtime {
            active: Some(active(3, None)),
            status: Status {
                phase: "transcribing".into(),
                text: "कल meeting है".into(),
                ..Status::default()
            },
            ..Runtime::default()
        };
        runtime.cancel_active();
        assert_eq!(runtime.status.phase, "idle");
        assert!(runtime.status.text.is_empty());
    }
}
