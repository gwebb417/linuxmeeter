#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, RwLock};
use std::time::Duration;

use lm_protocol::{
    AppState, BusId, CompParams, EngineCommand, EngineEvent, EqParams, FxKind, FxSends, FxState, GateParams,
    LimiterParams, MacroButton, PanParams, StripId, StripKind, Target, VbanRecv, VbanSend,
};
use tauri::{Emitter, Manager};

struct AppData {
    cmd_tx: pipewire::channel::Sender<EngineCommand>,
    state: Arc<RwLock<Option<AppState>>>,
}

fn send(data: &tauri::State<AppData>, cmd: EngineCommand) -> Result<(), String> {
    data.cmd_tx.send(cmd).map_err(|_| "engine unavailable".to_string())
}

#[tauri::command]
fn get_state(data: tauri::State<AppData>) -> Result<AppState, String> {
    // The engine builds its graph asynchronously at startup; wait briefly.
    for _ in 0..50 {
        if let Some(s) = data.state.read().expect("state lock").clone() {
            return Ok(s);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err("engine not ready".into())
}

#[tauri::command]
fn set_gain(data: tauri::State<AppData>, target: Target, db: f64) -> Result<(), String> {
    send(&data, EngineCommand::SetGain { target, db: db as f32 })
}

#[tauri::command]
fn set_mute(data: tauri::State<AppData>, target: Target, mute: bool) -> Result<(), String> {
    send(&data, EngineCommand::SetMute { target, mute })
}

#[tauri::command]
fn set_solo(data: tauri::State<AppData>, strip_id: StripId, solo: bool) -> Result<(), String> {
    send(&data, EngineCommand::SetSolo { strip: strip_id, solo })
}

#[tauri::command]
fn set_route(data: tauri::State<AppData>, strip_id: StripId, bus: BusId, on: bool) -> Result<(), String> {
    send(&data, EngineCommand::SetRoute { strip: strip_id, bus, on })
}

#[tauri::command]
fn set_gate_params(data: tauri::State<AppData>, strip_id: StripId, params: GateParams) -> Result<(), String> {
    send(&data, EngineCommand::SetGateParams { strip: strip_id, params })
}

#[tauri::command]
fn set_comp_params(data: tauri::State<AppData>, strip_id: StripId, params: CompParams) -> Result<(), String> {
    send(&data, EngineCommand::SetCompParams { strip: strip_id, params })
}

#[tauri::command]
fn set_eq_params(data: tauri::State<AppData>, strip_id: StripId, params: EqParams) -> Result<(), String> {
    send(&data, EngineCommand::SetEqParams { strip: strip_id, params })
}

#[tauri::command]
fn set_pan_params(data: tauri::State<AppData>, strip_id: StripId, params: PanParams) -> Result<(), String> {
    send(&data, EngineCommand::SetPanParams { strip: strip_id, params })
}

#[tauri::command]
fn set_sends(data: tauri::State<AppData>, strip_id: StripId, sends: FxSends) -> Result<(), String> {
    send(&data, EngineCommand::SetSends { strip: strip_id, sends })
}

#[tauri::command]
fn set_fx(data: tauri::State<AppData>, fx: FxState) -> Result<(), String> {
    send(&data, EngineCommand::SetFx { fx })
}

#[tauri::command]
fn set_fx_route(data: tauri::State<AppData>, fx: FxKind, bus: BusId, on: bool) -> Result<(), String> {
    send(&data, EngineCommand::SetFxRoute { fx, bus, on })
}

#[tauri::command]
fn set_macros(data: tauri::State<AppData>, macros: Vec<MacroButton>) -> Result<(), String> {
    send(&data, EngineCommand::SetMacros { macros })
}

#[tauri::command]
fn trigger_macro(data: tauri::State<AppData>, id: u32, pressed: bool) -> Result<(), String> {
    send(&data, EngineCommand::TriggerMacro { id, pressed })
}

#[tauri::command]
fn set_bus_eq_params(data: tauri::State<AppData>, bus: BusId, params: EqParams) -> Result<(), String> {
    send(&data, EngineCommand::SetBusEqParams { bus, params })
}

#[tauri::command]
fn set_limiter_params(data: tauri::State<AppData>, bus: BusId, params: LimiterParams) -> Result<(), String> {
    send(&data, EngineCommand::SetLimiterParams { bus, params })
}

#[tauri::command]
fn set_strip_device(data: tauri::State<AppData>, strip_id: StripId, hw_key: String) -> Result<(), String> {
    send(&data, EngineCommand::SetStripDevice { strip: strip_id, hw_key })
}

#[tauri::command]
fn set_bus_target(data: tauri::State<AppData>, bus: BusId, hw_key: String) -> Result<(), String> {
    send(&data, EngineCommand::SetBusTarget { bus, hw_key })
}

#[tauri::command]
fn set_bus_vban(data: tauri::State<AppData>, bus: BusId, config: VbanSend) -> Result<(), String> {
    send(&data, EngineCommand::SetBusVban { bus, config })
}

#[tauri::command]
fn add_vban_strip(data: tauri::State<AppData>, label: String, config: VbanRecv) -> Result<(), String> {
    send(&data, EngineCommand::AddVbanStrip { label, config })
}

#[tauri::command]
fn set_strip_vban(
    data: tauri::State<AppData>,
    strip_id: StripId,
    label: String,
    config: VbanRecv,
) -> Result<(), String> {
    send(&data, EngineCommand::SetStripVban { strip: strip_id, label, config })
}

#[tauri::command]
fn remove_strip(data: tauri::State<AppData>, strip_id: StripId) -> Result<(), String> {
    send(&data, EngineCommand::RemoveStrip { strip: strip_id })
}

#[tauri::command]
fn add_strip(data: tauri::State<AppData>, kind: StripKind, label: String) -> Result<(), String> {
    send(&data, EngineCommand::AddStrip { kind, label })
}

#[tauri::command]
fn add_bus(data: tauri::State<AppData>, output: bool) -> Result<(), String> {
    send(&data, EngineCommand::AddBus { output })
}

#[tauri::command]
fn remove_bus(data: tauri::State<AppData>, bus: BusId) -> Result<(), String> {
    send(&data, EngineCommand::RemoveBus { bus })
}

#[tauri::command]
fn set_label(data: tauri::State<AppData>, target: Target, label: String) -> Result<(), String> {
    send(&data, EngineCommand::SetLabel { target, label })
}

#[tauri::command]
fn set_default_output(data: tauri::State<AppData>, on: bool) -> Result<(), String> {
    send(&data, EngineCommand::SetDefaultOutput { on })
}

/// Start/stop meter frames to follow window visibility. Each frame costs a
/// script evaluation in the webview plus a canvas repaint per meter, so a
/// mixer parked in the tray should not be paying for either.
fn set_meters<R: tauri::Runtime, M: Manager<R>>(manager: &M, on: bool) {
    if let Some(data) = manager.try_state::<AppData>() {
        let _ = data.cmd_tx.send(EngineCommand::SetMetersEnabled { on });
    }
}

fn autostart_path() -> std::path::PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var_os("HOME").expect("HOME not set")).join(".config")
        })
        .join("autostart/linuxmeeter.desktop")
}

#[tauri::command]
fn get_autostart() -> bool {
    autostart_path().exists()
}

#[tauri::command]
fn set_autostart(on: bool) -> Result<(), String> {
    let path = autostart_path();
    if on {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let entry = format!(
            "[Desktop Entry]\nType=Application\nName=linuxmeeter\nComment=Virtual audio mixer\nExec={} --minimized\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
            exe.display()
        );
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, entry).map_err(|e| e.to_string())
    } else {
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let engine = lm_engine::engine::spawn();
    let cmd_tx = engine.cmd_tx.clone();
    let state: Arc<RwLock<Option<AppState>>> = Arc::new(RwLock::new(None));

    tauri::Builder::default()
        .manage(AppData { cmd_tx: engine.cmd_tx.clone(), state: state.clone() })
        .invoke_handler(tauri::generate_handler![
            get_state,
            set_gain,
            set_mute,
            set_solo,
            set_route,
            set_gate_params,
            set_comp_params,
            set_eq_params,
            set_pan_params,
            set_bus_eq_params,
            set_sends,
            set_fx,
            set_fx_route,
            set_macros,
            trigger_macro,
            set_limiter_params,
            set_strip_device,
            set_bus_target,
            set_bus_vban,
            add_vban_strip,
            set_strip_vban,
            remove_strip,
            add_strip,
            add_bus,
            remove_bus,
            set_label,
            set_default_output,
            get_autostart,
            set_autostart,
        ])
        .setup(move |app| {
            // Pump engine events into the webview.
            let handle = app.handle().clone();
            let evt_rx = engine.evt_rx;
            std::thread::Builder::new()
                .name("lm-event-pump".into())
                .spawn(move || {
                    while let Ok(evt) = evt_rx.recv() {
                        match evt {
                            EngineEvent::State(s) => {
                                *state.write().expect("state lock") = Some(s.clone());
                                let _ = handle.emit("state_changed", &s);
                            }
                            EngineEvent::Meters(f) => {
                                let _ = handle.emit("meters", &f);
                            }
                            EngineEvent::Error(msg) => {
                                let _ = handle.emit("engine_error", &msg);
                            }
                        }
                    }
                })
                .expect("spawn event pump");

            // Tray: closing the window hides it; the engine (and all virtual
            // devices) keeps running until Quit.
            let show = tauri::menu::MenuItem::with_id(app, "show", "Show mixer", true, None::<&str>)?;
            let quit = tauri::menu::MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = tauri::menu::Menu::with_items(app, &[&show, &quit])?;
            tauri::tray::TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().expect("window icon").clone())
                .tooltip("linuxmeeter")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.unminimize();
                            let _ = w.set_focus();
                            set_meters(app, true);
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            if std::env::args().any(|a| a == "--minimized") {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                    set_meters(app, false);
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Close-to-tray: keep the mixer (and its devices) alive, but
                // stop metering — nothing can see it.
                api.prevent_close();
                let _ = window.hide();
                set_meters(window, false);
            }
        })
        .build(tauri::generate_context!())
        .expect("error building linuxmeeter")
        .run(move |_app, event| {
            if let tauri::RunEvent::Exit = event {
                // Graceful engine shutdown -> modules unload -> zero lm.* nodes left.
                let _ = cmd_tx.send(EngineCommand::Shutdown);
                std::thread::sleep(Duration::from_millis(200));
            }
        });
}
