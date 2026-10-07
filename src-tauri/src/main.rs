#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use gharchive::{cli, db::Store, scheduler, Envelope};
use serde_json::{json, Value};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
async fn action(
    app: tauri::AppHandle,
    s: tauri::State<'_, Store>,
    name: String,
    payload: Value,
) -> std::result::Result<Envelope, String> {
    let result = if name == "update-check" || name == "update-install" {
        gharchive::update::action(&app, &s, name == "update-install", payload["force"] == true)
            .await
    } else if name == "github" {
        app.opener()
            .open_url(gharchive::REPOSITORY, None::<&str>)
            .map(|_| json!(true))
            .map_err(|e| gharchive::Failure::new(5, e))
    } else if name == "open-root" {
        match s.get("backup_root") {
            Ok(v) => {
                let p = v.as_str().unwrap();
                match std::fs::create_dir_all(p) {
                    Ok(_) => app
                        .opener()
                        .open_path(p, None::<&str>)
                        .map(|_| json!(true))
                        .map_err(|e| gharchive::Failure::new(5, e)),
                    Err(e) => Err(e.into()),
                }
            }
            Err(e) => Err(e),
        }
    } else if name == "set" && payload["key"] == "autostart" {
        let enabled = payload["value"] == true;
        let result = if enabled {
            app.autolaunch().enable()
        } else {
            app.autolaunch().disable()
        };
        match result {
            Ok(_) => s.set("autostart", json!(enabled)),
            Err(e) => Err(gharchive::Failure::new(2, e)),
        }
    } else {
        cli::action(&s, &name, payload).await
    };
    Ok(gharchive::response(result))
}
fn show(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}
fn main() {
    let s = match Store::open() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
    let _ = scheduler::recover(&s);
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show(app)))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name("GhArchive")
                .build(),
        )
        .manage(s)
        .invoke_handler(tauri::generate_handler![action])
        .setup(|app| {
            let state = app.state::<Store>();
            let enabled = state.get("autostart").ok().is_some_and(|v| v == true);
            if enabled {
                app.autolaunch().enable()?;
            } else {
                app.autolaunch().disable()?;
            }
            let lock = scheduler::daemon_lock(&state)?;
            let store = (*state).clone();
            tauri::async_runtime::spawn(scheduler::serve(store, lock));
            let show_item = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
            let all = MenuItem::with_id(app, "all", "立即备份全部", true, None::<&str>)?;
            let pause = MenuItem::with_id(app, "pause", "暂停 / 恢复定时", true, None::<&str>)?;
            let root = MenuItem::with_id(app, "root", "打开备份根目录", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &all, &pause, &root, &quit])?;
            TrayIconBuilder::with_id("gharchive-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("GhArchive · 仓库备份")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    let s = app.state::<Store>();
                    match event.id.as_ref() {
                        "show" => show(app),
                        "all" => {
                            let s = (*s).clone();
                            tauri::async_runtime::spawn(async move {
                                let _ = gharchive::backup::run_all(&s).await;
                            });
                        }
                        "pause" => {
                            let paused = s.get("paused").ok().is_some_and(|v| v == true);
                            let _ = s.set("paused", json!(!paused));
                        }
                        "root" => {
                            if let Ok(root) = s.get("backup_root") {
                                if let Some(p) = root.as_str() {
                                    let _ = std::fs::create_dir_all(p);
                                    let _ = app.opener().open_path(p, None::<&str>);
                                }
                            }
                        }
                        "quit" => {
                            let active = s
                                .status()
                                .ok()
                                .is_some_and(|v| v["running"].as_u64().unwrap_or(0) > 0);
                            if active {
                                show(app);
                            } else {
                                app.exit(0);
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;
            if state.get("start_minimized")? == true && state.get("accepted_notice")? == true {
                if let Some(w) = app.get_webview_window("main") {
                    w.hide()?;
                }
            }
            Ok(())
        })
        .on_window_event(|w, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = w.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("GhArchive 启动失败");
}
