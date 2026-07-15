#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{env, path::PathBuf, time::Duration};

use slint::winit_030::WinitWindowAccessor;
use slint::{ComponentHandle, PhysicalPosition, Timer};

slint::include_modules!();

fn requested_view() -> i32 {
    let args: Vec<String> = env::args().collect();
    let value = args
        .windows(2)
        .find(|pair| pair[0] == "--screen")
        .map(|pair| pair[1].as_str())
        .unwrap_or("send");

    match value {
        "receive" => 1,
        "transfer" => 2,
        _ => 0,
    }
}

fn capture_path() -> Option<PathBuf> {
    env::var_os("ECLIPXSE_CAPTURE").map(PathBuf::from)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let capture = capture_path();
    let mut backend = slint::BackendSelector::new().backend_name("winit".into());
    if capture.is_some() {
        backend = backend.renderer_name("software".into());
    }
    backend.select()?;

    let app = AppWindow::new()?;
    app.set_current_view(requested_view());

    let weak = app.as_weak();
    app.on_close_requested(move || {
        if let Some(app) = weak.upgrade() {
            let _ = app.window().hide();
        }
    });

    let weak = app.as_weak();
    app.on_minimize_requested(move || {
        if let Some(app) = weak.upgrade() {
            app.window().set_minimized(true);
        }
    });

    let weak = app.as_weak();
    app.on_maximize_requested(move || {
        if let Some(app) = weak.upgrade() {
            let window = app.window();
            window.set_maximized(!window.is_maximized());
        }
    });

    let weak = app.as_weak();
    app.on_drag_requested(move || {
        if let Some(app) = weak.upgrade() {
            app.window().with_winit_window(|window| {
                let _ = window.drag_window();
            });
        }
    });

    let weak = app.as_weak();
    app.on_center_requested(move || {
        let Some(app) = weak.upgrade() else { return };
        let Some((monitor_size, window_size)) = app
            .window()
            .with_winit_window(|window| {
                let monitor = window.current_monitor()?;
                Some((monitor.size(), window.outer_size()))
            })
            .flatten()
        else {
            return;
        };

        let x = monitor_size.width.saturating_sub(window_size.width) / 2;
        let y = monitor_size.height.saturating_sub(window_size.height) / 2;
        app.window()
            .set_position(PhysicalPosition::new(x as i32, y as i32));
    });

    if let Some(path) = capture {
        let weak = app.as_weak();
        Timer::single_shot(Duration::from_millis(900), move || {
            let Some(app) = weak.upgrade() else { return };
            let Ok(buffer) = app.window().take_snapshot() else {
                return;
            };
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = image::save_buffer(
                &path,
                buffer.as_bytes(),
                buffer.width(),
                buffer.height(),
                image::ColorType::Rgba8,
            );
            let _ = app.window().hide();
        });
    }

    app.run()?;
    Ok(())
}
