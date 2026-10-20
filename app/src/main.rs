slint::include_modules!();

use slint::{ModelRc, SharedString, VecModel};
use std::thread;

const WELCOME: &[&str] = &[
    "欢迎回来，主人~",
    "今天也要加油哦！",
    "一起去冒险吧~",
    "想你了，快来玩~",
    "新的一天，新的开始！",
    "准备好了吗？出发！",
    "方块世界在召唤你~",
    "今天想做什么呢？",
    "来陪我玩好不好~",
    "冒险者，启程吧！",
];

fn pick_welcome() -> SharedString {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as usize)
        .unwrap_or(0);
    WELCOME[n % WELCOME.len()].into()
}

fn update_launch_subtitle(ui: &AppWindow) {
    let version = ui.get_selected_version();
    let login = if ui.get_login_microsoft() {
        "正版登录"
    } else {
        "离线模式"
    };
    let game = if version.is_empty() {
        "请先下载游戏".to_string()
    } else {
        format!("Minecraft {version}")
    };
    ui.set_launch_subtitle(format!("{game}  {login}").into());
}

fn apply_java_list(ui: &AppWindow, lines: Vec<SharedString>) {
    let count = lines.len();
    ui.set_java_lines(ModelRc::new(VecModel::from(lines)));
    ui.set_java_searching(false);
    ui.set_java_summary(if count == 0 {
        "未找到可用的 Java，可在下方刷新或安装 JDK。".into()
    } else {
        format!("已检测到 {count} 个 Java。").into()
    });
}

fn run_java_search(ui_weak: slint::Weak<AppWindow>) {
    thread::spawn(move || {
        let roots = sakucra_java::default_search_roots();
        let entries = sakucra_java::search_java(&roots);
        let lines: Vec<SharedString> = entries
            .iter()
            .map(|e| e.display_line().into())
            .collect();

        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = ui_weak.upgrade() {
                apply_java_list(&ui, lines);
            }
        });
    });
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    ui.set_welcome_text(pick_welcome());
    update_launch_subtitle(&ui);

    let ui_weak = ui.as_weak();

    ui.on_username_changed({
        let ui_weak = ui_weak.clone();
        move |name| {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_username(name);
            }
        }
    });

    ui.on_open_settings({
        let ui_weak = ui_weak.clone();
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_current_page(1);
                ui.set_show_version_dropdown(false);
            }
        }
    });

    ui.on_open_versions({
        let ui_weak = ui_weak.clone();
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_current_page(2);
                ui.set_show_version_dropdown(false);
            }
        }
    });

    ui.on_back_home({
        let ui_weak = ui_weak.clone();
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_current_page(0);
            }
        }
    });

    ui.on_set_login_offline({
        let ui_weak = ui_weak.clone();
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_login_microsoft(false);
                update_launch_subtitle(&ui);
            }
        }
    });

    ui.on_set_login_microsoft({
        let ui_weak = ui_weak.clone();
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_login_microsoft(true);
                update_launch_subtitle(&ui);
            }
        }
    });

    ui.on_toggle_version_dropdown({
        let ui_weak = ui_weak.clone();
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_show_version_dropdown(!ui.get_show_version_dropdown());
            }
        }
    });

    ui.on_select_version({
        let ui_weak = ui_weak.clone();
        move |ver| {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_selected_version(ver);
                ui.set_show_version_dropdown(false);
                update_launch_subtitle(&ui);
            }
        }
    });

    ui.on_launch_game({
        let ui_weak = ui_weak.clone();
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                if ui.get_selected_version().is_empty() {
                    ui.set_launch_subtitle("请先下载或选择游戏版本".into());
                    return;
                }
                ui.set_launching(true);
                let weak = ui_weak.clone();
                thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(800));
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = weak.upgrade() {
                            ui.set_launching(false);
                            ui.set_launch_subtitle(
                                "游戏启动模块开发中".into(),
                            );
                        }
                    });
                });
            }
        }
    });

    ui.on_refresh_java({
        let ui_weak = ui_weak.clone();
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_java_searching(true);
                ui.set_java_summary("正在搜索本机 Java…".into());
            }
            run_java_search(ui_weak.clone());
        }
    });

    ui.on_set_settings_tab({
        let ui_weak = ui_weak.clone();
        move |tab| {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_settings_tab(tab);
            }
        }
    });

    ui.on_minimize_window(move || {
        // Slint 跨平台最小化由窗口管理器处理；后续可接 winit。
    });

    ui.on_close_window(move || {
        slint::quit_event_loop().ok();
    });

    run_java_search(ui_weak);
    ui.run()
}
