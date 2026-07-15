#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backend;

use std::{env, path::PathBuf, thread, time::Duration};

use backend::{BackendSnapshot, BeamBackend, TransferDirection, TransferInfo, TransferStatus};
use qrcode::{QrCode, types::Color as QrColor};
use slint::winit_030::WinitWindowAccessor;
use slint::{
    ComponentHandle, Image as SlintImage, ModelRc, PhysicalPosition, Rgba8Pixel, SharedPixelBuffer,
    Timer, VecModel,
};

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

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes == 0 {
        return "0 B".into();
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 || value >= 10.0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn file_kind(name: &str) -> i32 {
    let extension = std::path::Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(extension.as_str(), "mov" | "mp4" | "mkv" | "avi" | "webm") {
        0
    } else if matches!(extension.as_str(), "zip" | "7z" | "rar" | "tar" | "gz") {
        1
    } else {
        2
    }
}

fn transfer_status_text(transfer: &TransferInfo) -> String {
    if let Some(error) = &transfer.error {
        return error.clone();
    }
    match (transfer.direction, transfer.status) {
        (TransferDirection::Sending, TransferStatus::Waiting) => "Waiting for phone".into(),
        (TransferDirection::Sending, TransferStatus::Transferring) => "Sending now".into(),
        (TransferDirection::Sending, TransferStatus::Complete) => "Sent successfully".into(),
        (TransferDirection::Receiving, TransferStatus::Waiting) => "Waiting".into(),
        (TransferDirection::Receiving, TransferStatus::Transferring) => "Receiving now".into(),
        (TransferDirection::Receiving, TransferStatus::Complete) => "Saved to Downloads".into(),
        (_, TransferStatus::Failed) => "Transfer failed".into(),
    }
}

fn apply_snapshot(app: &AppWindow, snapshot: BackendSnapshot) {
    let total_size = snapshot.files.iter().map(|file| file.size).sum::<u64>();
    let file_entries = snapshot
        .files
        .iter()
        .map(|file| FileEntry {
            name: file.name.clone().into(),
            size: format_bytes(file.size).into(),
            kind: file_kind(&file.name),
        })
        .collect::<Vec<_>>();
    app.set_selected_files(ModelRc::new(VecModel::from(file_entries)));
    app.set_has_files(!snapshot.files.is_empty());
    app.set_selection_summary(if snapshot.files.is_empty() {
        "No files selected".into()
    } else {
        format!(
            "{} {} · {}",
            snapshot.files.len(),
            if snapshot.files.len() == 1 {
                "file"
            } else {
                "files"
            },
            format_bytes(total_size)
        )
        .into()
    });

    let connected = snapshot.device_name.is_some();
    let device_name = snapshot
        .device_name
        .clone()
        .unwrap_or_else(|| "Phone companion".into());
    app.set_device_connected(connected);
    app.set_device_name(device_name.clone().into());
    app.set_device_detail(if connected {
        "Same Wi-Fi · Connected".into()
    } else {
        "Open Receive · Scan QR".into()
    });

    let transfer_entries = snapshot
        .transfers
        .iter()
        .map(|transfer| {
            let status = transfer_status_text(transfer);
            TransferEntry {
                name: transfer.name.clone().into(),
                detail: format!("{} · {status}", format_bytes(transfer.size)).into(),
                percent: match transfer.status {
                    TransferStatus::Complete => "Done".into(),
                    TransferStatus::Failed => "Failed".into(),
                    _ => format!("{:.0}%", transfer.progress).into(),
                },
                progress: transfer.progress / 100.0,
                kind: file_kind(&transfer.name),
            }
        })
        .collect::<Vec<_>>();
    app.set_transfers(ModelRc::new(VecModel::from(transfer_entries)));

    let complete_count = snapshot
        .transfers
        .iter()
        .filter(|transfer| transfer.status == TransferStatus::Complete)
        .count();
    let active_count = snapshot
        .transfers
        .iter()
        .filter(|transfer| transfer.status == TransferStatus::Transferring)
        .count();
    let has_received_transfer = snapshot
        .transfers
        .iter()
        .any(|transfer| transfer.direction == TransferDirection::Receiving);
    let weighted_size = snapshot
        .transfers
        .iter()
        .map(|transfer| transfer.size.max(1))
        .sum::<u64>();
    let weighted_progress = if weighted_size == 0 {
        0.0
    } else {
        snapshot
            .transfers
            .iter()
            .map(|transfer| transfer.progress as f64 * transfer.size.max(1) as f64)
            .sum::<f64>()
            / weighted_size as f64
    };
    app.set_overall_progress((weighted_progress / 100.0) as f32);
    app.set_overall_percent(format!("{weighted_progress:.0}%").into());
    app.set_queue_meta(if snapshot.transfers.is_empty() {
        "No transfers".into()
    } else {
        format!("{complete_count} of {} complete", snapshot.transfers.len()).into()
    });

    if let Some(error) = snapshot.last_error {
        app.set_status_text("Needs attention".into());
        app.set_overall_title("Transfer needs attention".into());
        app.set_overall_detail(error.into());
    } else if !snapshot.transfers.is_empty() && complete_count == snapshot.transfers.len() {
        app.set_status_text("Complete".into());
        app.set_overall_title("Everything arrived".into());
        app.set_overall_detail(
            format!("{} transfer(s) completed", snapshot.transfers.len()).into(),
        );
    } else if active_count > 0 {
        app.set_status_text("Transferring".into());
        app.set_overall_title(format!("Connected to {device_name}").into());
        app.set_overall_detail(format!("{active_count} active · {complete_count} complete").into());
    } else if connected {
        app.set_status_text("Connected".into());
        app.set_overall_title(format!("Ready for {device_name}").into());
        app.set_overall_detail("Choose a file on either device to begin".into());
    } else {
        app.set_status_text("Ready".into());
        app.set_overall_title("Waiting for your phone".into());
        app.set_overall_detail("Open Receive and scan the QR code".into());
    }

    if app.get_current_view() != 2 && (active_count > 0 || has_received_transfer) {
        app.set_current_view(2);
    }
}

fn qr_image(value: &str) -> Result<SlintImage, qrcode::types::QrError> {
    const TARGET: u32 = 226;
    const QUIET_MODULES: usize = 4;
    let code = QrCode::new(value.as_bytes())?;
    let modules = code.width();
    let scale = (TARGET as usize / (modules + QUIET_MODULES * 2)).max(1);
    let drawn = modules * scale;
    let offset = (TARGET as usize - drawn) / 2;
    let mut pixels = SharedPixelBuffer::<Rgba8Pixel>::new(TARGET, TARGET);
    for pixel in pixels.make_mut_slice() {
        *pixel = Rgba8Pixel::new(247, 241, 222, 255);
    }
    for y in 0..modules {
        for x in 0..modules {
            if code[(x, y)] != QrColor::Dark {
                continue;
            }
            for py in 0..scale {
                for px in 0..scale {
                    let target_x = offset + x * scale + px;
                    let target_y = offset + y * scale + py;
                    pixels.make_mut_slice()[target_y * TARGET as usize + target_x] =
                        Rgba8Pixel::new(18, 10, 6, 255);
                }
            }
        }
    }
    Ok(SlintImage::from_rgba8(pixels))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let capture = capture_path();
    let mut backend_selector = slint::BackendSelector::new().backend_name("winit".into());
    if capture.is_some() {
        backend_selector = backend_selector.renderer_name("software".into());
    }
    backend_selector.select()?;

    let (beam_backend, backend_events) = BeamBackend::start()?;
    let app = AppWindow::new()?;
    app.set_current_view(requested_view());
    app.set_pairing_code(beam_backend.pairing_code().into());
    app.set_pairing_url(beam_backend.pairing_url().into());
    app.set_lan_reachable(beam_backend.lan_reachable());
    app.set_pairing_qr(qr_image(beam_backend.pairing_url())?);
    apply_snapshot(&app, beam_backend.snapshot());

    let weak = app.as_weak();
    let event_backend = beam_backend.clone();
    thread::Builder::new()
        .name("beam-ui-events".into())
        .spawn(move || {
            while backend_events.recv().is_ok() {
                let snapshot = event_backend.snapshot();
                let _ = weak.upgrade_in_event_loop(move |app| apply_snapshot(&app, snapshot));
            }
        })?;

    let picker_backend = beam_backend.clone();
    app.on_choose_files(move || {
        if let Some(paths) = rfd::FileDialog::new()
            .set_title("Choose files to beam")
            .pick_files()
        {
            picker_backend.add_files(paths);
        }
    });

    let clear_backend = beam_backend.clone();
    app.on_clear_files(move || clear_backend.clear_files());

    let remove_backend = beam_backend.clone();
    app.on_remove_file(move |index| {
        if index >= 0 {
            remove_backend.remove_file(index as usize);
        }
    });

    app.on_start_transfer(|| {});

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

#[cfg(test)]
mod tests {
    use super::{file_kind, format_bytes};

    #[test]
    fn formats_file_sizes_for_the_ui() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1_572_864), "1.5 MB");
    }

    #[test]
    fn chooses_visual_file_kinds() {
        assert_eq!(file_kind("clip.mp4"), 0);
        assert_eq!(file_kind("bundle.zip"), 1);
        assert_eq!(file_kind("photo.png"), 2);
    }
}
