use tauri::{image::Image, tray::TrayIcon, AppHandle, Manager, Wry};

use crate::{updater::UpdateCheckResult, AppState};

use super::{default_tooltip, snapshot_tooltip};

pub(super) fn update_indicator(
    app: &AppHandle,
    tray: &TrayIcon<Wry>,
    update: Option<&UpdateCheckResult>,
) {
    let base_icon = base_tray_icon(app);
    let icon = if update.is_some_and(|result| result.available) {
        icon_with_update_badge(base_icon)
    } else {
        base_icon
    };
    let _ = tray.set_icon(Some(icon));
    #[cfg(target_os = "macos")]
    let _ = tray.set_icon_as_template(false);

    let usage = app
        .try_state::<AppState>()
        .and_then(|state| state.current_snapshot())
        .map(|snapshot| snapshot_tooltip(&snapshot))
        .unwrap_or_else(|| default_tooltip().to_string());
    let _ = tray.set_tooltip(Some(tooltip_with_update(usage, update)));
}

#[cfg(target_os = "macos")]
pub(super) fn base_tray_icon(_app: &AppHandle) -> Image<'static> {
    macos_tray_icon()
}

#[cfg(not(target_os = "macos"))]
pub(super) fn base_tray_icon(app: &AppHandle) -> Image<'static> {
    app.default_window_icon()
        .cloned()
        .map(Image::to_owned)
        .unwrap_or_else(|| {
            Image::from_bytes(include_bytes!("../../icons/tray.png")).expect("tray icon")
        })
}

#[cfg(any(target_os = "macos", test))]
fn macos_tray_icon() -> Image<'static> {
    const SIZE: u32 = 36;
    let source = Image::from_bytes(include_bytes!("../../icons/tray.png")).expect("tray icon");
    let source_width = source.width();
    let source_height = source.height();
    let source_rgba = source.rgba();
    let mut rgba = vec![0; (SIZE * SIZE * 4) as usize];

    for y in 0..SIZE {
        for x in 0..SIZE {
            let start_x = x * source_width / SIZE;
            let end_x = ((x + 1) * source_width).div_ceil(SIZE);
            let start_y = y * source_height / SIZE;
            let end_y = ((y + 1) * source_height).div_ceil(SIZE);
            let mut alpha_sum = 0u64;
            let mut red_sum = 0u64;
            let mut green_sum = 0u64;
            let mut blue_sum = 0u64;
            let mut sample_count = 0u64;

            for source_y in start_y..end_y.min(source_height) {
                for source_x in start_x..end_x.min(source_width) {
                    let source_offset = ((source_y * source_width + source_x) * 4) as usize;
                    let pixel = &source_rgba[source_offset..source_offset + 4];
                    let alpha = pixel[3] as u64;
                    alpha_sum += alpha;
                    red_sum += pixel[0] as u64 * alpha;
                    green_sum += pixel[1] as u64 * alpha;
                    blue_sum += pixel[2] as u64 * alpha;
                    sample_count += 1;
                }
            }

            let offset = ((y * SIZE + x) * 4) as usize;
            if alpha_sum > 0 {
                rgba[offset] = (red_sum / alpha_sum) as u8;
                rgba[offset + 1] = (green_sum / alpha_sum) as u8;
                rgba[offset + 2] = (blue_sum / alpha_sum) as u8;
            }
            rgba[offset + 3] = (alpha_sum / sample_count.max(1)) as u8;
        }
    }

    Image::new_owned(rgba, SIZE, SIZE)
}

pub(super) fn tooltip_with_update(usage: String, update: Option<&UpdateCheckResult>) -> String {
    match update.filter(|result| result.available) {
        Some(result) => match result.version.as_deref() {
            Some(version) => format!("发现新版本 v{}\n{}", version, usage),
            None => format!("发现新版本\n{}", usage),
        },
        None => usage,
    }
}

fn icon_with_update_badge(icon: Image<'static>) -> Image<'static> {
    let width = icon.width();
    let height = icon.height();
    let mut rgba = icon.rgba().to_vec();
    let size = width.min(height) as i32;
    let outer_radius = (size * 13 / 100).max(2);
    let inner_radius = (outer_radius * 72 / 100).max(1);
    let inset = (size * 4 / 100).max(1);
    let center_x = width as i32 - outer_radius - inset;
    let center_y = outer_radius + inset;

    // 深色描边让提示点在浅色和深色任务栏上都保持清晰。
    fill_circle(
        &mut rgba,
        width,
        height,
        center_x,
        center_y,
        outer_radius,
        [24, 32, 40, 242],
    );
    fill_circle(
        &mut rgba,
        width,
        height,
        center_x,
        center_y,
        inner_radius,
        [112, 235, 198, 255],
    );
    Image::new_owned(rgba, width, height)
}

fn fill_circle(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    center_x: i32,
    center_y: i32,
    radius: i32,
    color: [u8; 4],
) {
    let min_x = (center_x - radius).max(0);
    let max_x = (center_x + radius).min(width as i32 - 1);
    let min_y = (center_y - radius).max(0);
    let max_y = (center_y + radius).min(height as i32 - 1);
    let radius_squared = radius * radius;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = x - center_x;
            let dy = y - center_y;
            if dx * dx + dy * dy > radius_squared {
                continue;
            }
            let offset = ((y as u32 * width + x as u32) * 4) as usize;
            rgba[offset..offset + 4].copy_from_slice(&color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_update_version_to_tooltip() {
        let update = UpdateCheckResult {
            available: true,
            version: Some("0.0.7".to_string()),
            message: String::new(),
        };

        assert_eq!(
            tooltip_with_update("5h: 剩余 80%".to_string(), Some(&update)),
            "发现新版本 v0.0.7\n5h: 剩余 80%"
        );
    }

    #[test]
    fn draws_mint_update_badge_without_changing_canvas_size() {
        let icon = Image::new_owned(vec![0; 32 * 32 * 4], 32, 32);
        let badged = icon_with_update_badge(icon);

        assert_eq!((badged.width(), badged.height()), (32, 32));
        assert!(badged
            .rgba()
            .chunks_exact(4)
            .any(|pixel| pixel == [112, 235, 198, 255]));
    }

    #[test]
    fn keeps_windows_tray_colors_in_macos_icon() {
        let icon = macos_tray_icon();
        let pixels = icon.rgba().chunks_exact(4);
        let has_dark_background = pixels
            .clone()
            .any(|pixel| pixel[3] > 180 && pixel[0].max(pixel[1]).max(pixel[2]) < 80);
        let has_mint_ring = pixels
            .clone()
            .any(|pixel| pixel[3] > 180 && pixel[1] > 170 && pixel[1] > pixel[0]);
        let has_white_g = pixels
            .clone()
            .any(|pixel| pixel[3] > 180 && pixel[0].min(pixel[1]).min(pixel[2]) > 200);
        let has_orange_dot = pixels
            .clone()
            .any(|pixel| pixel[3] > 180 && pixel[0] > 200 && pixel[1] > 100 && pixel[2] < 140);

        assert_eq!((icon.width(), icon.height()), (36, 36));
        assert!(has_dark_background);
        assert!(has_mint_ring);
        assert!(has_white_g);
        assert!(has_orange_dot);
    }
}
