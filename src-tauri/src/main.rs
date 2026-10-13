#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod games;
mod memory;
mod monitor;
mod settings;
mod sysinfo;
mod tweaks;
mod util;

use serde::Serialize;
use settings::Settings;
use std::sync::{Arc, Mutex};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

const TRAY_OFF: &[u8] = include_bytes!("../icons/tray-off.png");
const TRAY_ON: &[u8] = include_bytes!("../icons/tray-on.png");

struct Core {
    settings: Settings,
    busy: bool,
    reboot: bool,
    system: Option<sysinfo::System>,
}

type Shared = Arc<Mutex<Core>>;
struct TrayToggle(MenuItem<Wry>);

fn core(app: &AppHandle) -> Shared {
    app.state::<Shared>().inner().clone()
}

fn live(app: &AppHandle) -> monitor::Shared {
    app.state::<monitor::Shared>().inner().clone()
}

fn save(app: &AppHandle) {
    settings::save(&core(app).lock().unwrap().settings);
}

fn refresh_tray(app: &AppHandle) {
    let on = core(app).lock().unwrap().settings.boost;
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_icon(Image::from_bytes(if on { TRAY_ON } else { TRAY_OFF }).ok());
        let _ = tray.set_tooltip(Some(if on { "Arweak · Boost aktif" } else { "Arweak" }));
    }
    if let Some(t) = app.try_state::<TrayToggle>() {
        let _ = t.0.set_text(if on { "Boost'u kapat" } else { "Boost" });
    }
}

fn show_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn set_logon_task(on: bool) {
    if on {
        let exe = std::env::current_exe().unwrap_or_default();
        let _ = util::run(
            "schtasks.exe",
            &format!(r#"/Create /TN "Arweak" /TR "\"{}\" --tray" /SC ONLOGON /RL HIGHEST /F"#, exe.display()),
        );
    } else {
        let _ = util::run("schtasks.exe", r#"/Delete /TN "Arweak" /F"#);
    }
}

fn boost_on(app: &AppHandle) {
    let (selected, mut guid) = {
        let c = core(app);
        let mut c = c.lock().unwrap();
        if c.busy || c.settings.boost {
            return;
        }
        c.busy = true;
        (c.settings.tweaks.clone(), c.settings.ultimate_guid.clone())
    };
    let reboot = tweaks::apply_selected(&selected, &mut guid);
    {
        let c = core(app);
        let mut c = c.lock().unwrap();
        c.settings.ultimate_guid = guid;
        c.settings.boost = true;
        c.busy = false;
        c.reboot |= reboot;
        c.system = None;
    }
    save(app);
    refresh_tray(app);
}

fn boost_off(app: &AppHandle) {
    {
        let c = core(app);
        let mut c = c.lock().unwrap();
        if c.busy {
            return;
        }
        c.busy = true;
    }
    let reboot = tweaks::revert_all();
    {
        let c = core(app);
        let mut c = c.lock().unwrap();
        c.settings.boost = false;
        c.busy = false;
        c.reboot |= reboot;
        c.system = None;
    }
    save(app);
    refresh_tray(app);
}

#[derive(Serialize)]
struct GameView {
    id: &'static str,
    name: &'static str,
    img: &'static str,
    running: bool,
}

#[derive(Serialize)]
struct StateView {
    boost: bool,
    busy: bool,
    reboot: bool,
    active: usize,
    total: usize,
    score: Option<u32>,
    game: GameView,
    fps: Option<u32>,
    low: Option<u32>,
    stutters: Option<u32>,
    frametime: Option<f32>,
    history: Vec<f32>,
    cpu: u32,
    gpu: u32,
    ram: u32,
    before: Option<u32>,
    after: Option<u32>,
}

fn active_count(c: &Core) -> usize {
    if !c.settings.boost {
        return 0;
    }
    let runtime = tweaks::TWEAKS
        .iter()
        .filter(|t| t.runtime && c.settings.tweaks.get(t.id).copied().unwrap_or(false))
        .count();
    tweaks::applied_count() + runtime
}

#[tauri::command]
fn get_state(app: AppHandle) -> StateView {
    let c = core(&app);
    let c = c.lock().unwrap();
    let l = live(&app);
    let l = l.lock().unwrap();
    let running = l.game.as_ref().map(|g| g.id);
    let gid = running.unwrap_or(c.settings.selected_game.as_str());
    let g = games::by_id(gid).unwrap_or(&games::GAMES[0]);
    let active = active_count(&c);
    let total = tweaks::TWEAKS.len();
    let stats = c.settings.stats.get(g.id).cloned().unwrap_or_default();
    let fps = l.game.as_ref().and_then(|_| monitor::fps_now(&l));
    StateView {
        boost: c.settings.boost,
        busy: c.busy,
        reboot: c.reboot,
        active,
        total,
        score: c.system.as_ref().map(|s| sysinfo::score(s, active, total)),
        game: GameView { id: g.id, name: g.name, img: g.img, running: running.is_some() },
        fps,
        low: monitor::low_1(&l),
        stutters: monitor::stutters(&l),
        frametime: l.history.back().copied(),
        history: l.history.iter().copied().collect(),
        cpu: l.cpu,
        gpu: l.gpu,
        ram: l.ram,
        before: stats.off,
        after: if c.settings.boost && running.is_some() { fps.or(stats.on) } else { stats.on },
    }
}

#[tauri::command]
async fn toggle_boost(app: AppHandle) {
    let on = core(&app).lock().unwrap().settings.boost;
    let a = app.clone();
    tauri::async_runtime::spawn_blocking(move || if on { boost_off(&a) } else { boost_on(&a) })
        .await
        .ok();
}

#[derive(Serialize)]
struct GameItem {
    id: &'static str,
    name: &'static str,
    img: &'static str,
    installed: bool,
    profile: bool,
    selected: bool,
}

#[tauri::command]
async fn get_games(app: AppHandle) -> Vec<GameItem> {
    let (profiles, selected) = {
        let c = core(&app);
        let c = c.lock().unwrap();
        (c.settings.profiles.clone(), c.settings.selected_game.clone())
    };
    tauri::async_runtime::spawn_blocking(move || {
        games::GAMES
            .iter()
            .map(|g| {
                let installed = games::installed(g);
                GameItem {
                    id: g.id,
                    name: g.name,
                    img: g.img,
                    installed,
                    profile: profiles.get(g.id).copied().unwrap_or(installed),
                    selected: selected == g.id,
                }
            })
            .collect()
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
fn set_profile(app: AppHandle, id: String, on: bool) {
    core(&app).lock().unwrap().settings.profiles.insert(id, on);
    save(&app);
}

#[tauri::command]
fn select_game(app: AppHandle, id: String) {
    core(&app).lock().unwrap().settings.selected_game = id;
    save(&app);
}

#[derive(Serialize)]
struct TweakItem {
    id: &'static str,
    name: &'static str,
    group: &'static str,
    on: bool,
}

#[tauri::command]
fn get_tweaks(app: AppHandle) -> Vec<TweakItem> {
    let c = core(&app);
    let c = c.lock().unwrap();
    tweaks::TWEAKS
        .iter()
        .map(|t| TweakItem { id: t.id, name: t.name, group: t.group, on: c.settings.tweaks.get(t.id).copied().unwrap_or(false) })
        .collect()
}

#[tauri::command]
async fn set_tweak(app: AppHandle, id: String, on: bool) {
    let boost = {
        let c = core(&app);
        let mut c = c.lock().unwrap();
        c.settings.tweaks.insert(id.clone(), on);
        c.settings.boost
    };
    save(&app);
    if !boost || tweaks::get(&id).is_none_or(|t| t.runtime) {
        return;
    }
    let a = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let reboot = if on {
            let (sel, mut guid) = {
                let c = core(&a);
                let c = c.lock().unwrap();
                (c.settings.tweaks.clone(), c.settings.ultimate_guid.clone())
            };
            let r = tweaks::apply_selected(&sel, &mut guid);
            core(&a).lock().unwrap().settings.ultimate_guid = guid;
            r
        } else {
            tweaks::revert_one(&id)
        };
        let c = core(&a);
        let mut c = c.lock().unwrap();
        c.reboot |= reboot;
        c.system = None;
    })
    .await
    .ok();
    save(&app);
}

#[tauri::command]
async fn set_all_tweaks(app: AppHandle, recommended: bool) {
    let ids: Vec<&str> = tweaks::TWEAKS.iter().map(|t| t.id).collect();
    for id in ids {
        let on = recommended && tweaks::get(id).is_some_and(|t| t.recommended);
        let cur = core(&app).lock().unwrap().settings.tweaks.get(id).copied().unwrap_or(false);
        if cur != on {
            set_tweak(app.clone(), id.to_string(), on).await;
        }
    }
}

#[tauri::command]
async fn get_system(app: AppHandle, fresh: bool) -> sysinfo::System {
    if !fresh {
        if let Some(s) = core(&app).lock().unwrap().system.clone() {
            return s;
        }
    }
    let s = tauri::async_runtime::spawn_blocking(sysinfo::read).await.unwrap_or_default();
    core(&app).lock().unwrap().system = Some(s.clone());
    s
}

#[tauri::command]
async fn clean_ram(app: AppHandle) -> i64 {
    let freed = tauri::async_runtime::spawn_blocking(memory::clean).await.unwrap_or(0);
    core(&app).lock().unwrap().system = None;
    freed.max(0)
}

#[tauri::command]
fn get_settings(app: AppHandle) -> serde_json::Value {
    let c = core(&app);
    let c = c.lock().unwrap();
    serde_json::json!({
        "start_with_windows": c.settings.start_with_windows,
        "auto_boost": c.settings.auto_boost,
        "close_to_tray": c.settings.close_to_tray,
    })
}

#[tauri::command]
fn set_setting(app: AppHandle, key: String, value: bool) {
    {
        let c = core(&app);
        let mut c = c.lock().unwrap();
        match key.as_str() {
            "start_with_windows" => {
                c.settings.start_with_windows = value;
                set_logon_task(value);
            }
            "auto_boost" => c.settings.auto_boost = value,
            "close_to_tray" => c.settings.close_to_tray = value,
            _ => return,
        }
    }
    save(&app);
}

#[tauri::command]
async fn restore_point() -> bool {
    tauri::async_runtime::spawn_blocking(|| {
        let out = util::ps(
            "try { Enable-ComputerRestore -Drive \"$env:SystemDrive\\\" -ErrorAction SilentlyContinue; Checkpoint-Computer -Description 'Arweak' -RestorePointType MODIFY_SETTINGS -ErrorAction Stop; 'ok' } catch { 'fail' }",
        );
        out.contains("ok")
    })
    .await
    .unwrap_or(false)
}

#[tauri::command]
async fn revert_everything(app: AppHandle) {
    let a = app.clone();
    tauri::async_runtime::spawn_blocking(move || boost_off(&a)).await.ok();
}

#[tauri::command]
fn close_window(app: AppHandle) {
    let to_tray = core(&app).lock().unwrap().settings.close_to_tray;
    if to_tray {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.hide();
        }
    } else {
        app.exit(0);
    }
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Arweak'i aç", true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "Boost", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Çık", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &toggle, &PredefinedMenuItem::separator(app)?, &quit])?;
    app.manage(TrayToggle(toggle));
    TrayIconBuilder::with_id("main")
        .icon(Image::from_bytes(TRAY_OFF)?)
        .tooltip("Arweak")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_window(app),
            "toggle" => {
                let a = app.clone();
                std::thread::spawn(move || {
                    if core(&a).lock().unwrap().settings.boost { boost_off(&a) } else { boost_on(&a) }
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn main() {
    let shared: Shared = Arc::new(Mutex::new(Core { settings: settings::load(), busy: false, reboot: false, system: None }));
    let live_state: monitor::Shared = Arc::new(Mutex::new(monitor::Live::default()));

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show_window(app)))
        .manage(shared)
        .manage(live_state)
        .setup(|app| {
            let handle = app.handle().clone();
            build_tray(&handle)?;
            refresh_tray(&handle);
            set_logon_task(core(&handle).lock().unwrap().settings.start_with_windows);

            let h1 = handle.clone();
            let h2 = handle.clone();
            let h3 = handle.clone();
            monitor::spawn(
                live(&handle),
                move || {
                    let c = core(&h1);
                    let c = c.lock().unwrap();
                    let on = |id: &str| c.settings.tweaks.get(id).copied().unwrap_or(false);
                    monitor::Options { boost: c.settings.boost, prio: on("prio"), bg: on("bg"), ram: on("ram") }
                },
                move |game, ended| {
                    if let Some((id, avg, on)) = ended {
                        let c = core(&h2);
                        let mut c = c.lock().unwrap();
                        let e = c.settings.stats.entry(id.to_string()).or_default();
                        if on { e.on = Some(avg) } else { e.off = Some(avg) }
                        settings::save(&c.settings);
                    }
                    if game.is_some() {
                        let auto = {
                            let c = core(&h2);
                            let c = c.lock().unwrap();
                            c.settings.auto_boost && !c.settings.boost
                        };
                        if auto {
                            boost_on(&h2);
                        }
                    }
                },
                move |id| {
                    let c = core(&h3);
                    let c = c.lock().unwrap();
                    c.settings.profiles.get(id).copied().unwrap_or(true)
                },
            );

            if !std::env::args().any(|a| a == "--tray") {
                show_window(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                close_window(window.app_handle().clone());
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            toggle_boost,
            get_games,
            set_profile,
            select_game,
            get_tweaks,
            set_tweak,
            set_all_tweaks,
            get_system,
            clean_ram,
            get_settings,
            set_setting,
            restore_point,
            revert_everything,
            close_window
        ])
        .build(tauri::generate_context!())
        .expect("Arweak")
        .run(|_, event| {
            if let tauri::RunEvent::Exit = event {
                monitor::kill_all();
            }
        });
}
