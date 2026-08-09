//! StatusNotifierItem tray via `ksni` (pure-Rust, over zbus). We talk SNI
//! directly instead of going through Tauri's libayatana-appindicator tray,
//! which fails to register on COSMIC.
//!
//! On COSMIC we prioritize **text over color**: the panel item is the Claude
//! logo followed by the same string the terminal statusLine shows, e.g.
//! `5h: 48.0% | 7d: 57.0%`. SNI has no reliable inline-text channel across
//! panels, so we render the text *into* the icon pixmap (a wide ARGB image),
//! which every SNI host displays. Text is drawn white with a dark 1px outline
//! so it stays legible on both light and dark panels.
//!
//! The poller feeds us the local machine's 5h / 7d quota via [`set_quota`].

use anyhow::Result;
use std::sync::{mpsc, Mutex, OnceLock};

use ksni::blocking::TrayMethods;
use ksni::{Icon, Tray};
use tauri::{AppHandle, Manager, Runtime};

/// The Claude logo shown at the left of the tray item (from the macOS repo).
const CLAUDE_PNG: &[u8] = include_bytes!("../icons/claudecode.png");

const PANEL_H: usize = 22; // rendered pixmap height
const LOGO_H: usize = 18; // logo box (vertically centered in PANEL_H)
const FONT_SCALE: usize = 2; // 5x7 glyphs scaled -> 10x14
const GLYPH_W: usize = 5;
const GLYPH_H: usize = 7;
const GAP: usize = 4; // px between logo and text

/// Latest local 5h / 7d usage % (0-100). Drives the rendered text.
static QUOTA: Mutex<(Option<f64>, Option<f64>)> = Mutex::new((None, None));
/// Wakes the tray thread to re-render after [`set_quota`].
static NUDGE: OnceLock<mpsc::Sender<()>> = OnceLock::new();

/// Set the local machine's 5h / 7d usage the tray reflects, and re-render.
/// Called by the poller each cycle with the *local* source's quota so the tray
/// text matches what Claude Code prints in that machine's terminal statusLine.
pub fn set_quota(five_hour: Option<f64>, seven_day: Option<f64>) {
    *QUOTA.lock().expect("QUOTA poisoned") = (five_hour, seven_day);
    if let Some(tx) = NUDGE.get() {
        let _ = tx.send(());
    }
}

fn current_text() -> String {
    let (five, seven) = *QUOTA.lock().expect("QUOTA poisoned");
    text_for(five, seven)
}

/// `5h: 48.0% | 7d: 57.0%`, dropping whichever window has no data. Empty when
/// neither is known yet (tray then shows just the logo).
fn text_for(five: Option<f64>, seven: Option<f64>) -> String {
    let mut parts = Vec::new();
    if let Some(p) = five {
        parts.push(format!("5h: {p:.1}%"));
    }
    if let Some(p) = seven {
        parts.push(format!("7d: {p:.1}%"));
    }
    parts.join(" | ")
}

// ── Pixmap composition ──────────────────────────────────────────────────────

/// The Claude logo decoded once and scaled to LOGO_H×LOGO_H ARGB.
fn logo() -> &'static (usize, Vec<u8>) {
    static LOGO: OnceLock<(usize, Vec<u8>)> = OnceLock::new();
    LOGO.get_or_init(|| {
        let decoder = png::Decoder::new(CLAUDE_PNG);
        let mut reader = decoder.read_info().expect("claude png header");
        let mut buf = vec![0u8; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).expect("claude png decode");
        let (sw, sh) = (info.width as usize, info.height as usize);
        let line = info.line_size;
        // Nearest-neighbor downscale to LOGO_H×LOGO_H, RGBA -> ARGB.
        let mut argb = vec![0u8; LOGO_H * LOGO_H * 4];
        for dy in 0..LOGO_H {
            for dx in 0..LOGO_H {
                let sx = dx * sw / LOGO_H;
                let sy = dy * sh / LOGO_H;
                let si = sy * line + sx * 4;
                let (r, g, b, a) = (buf[si], buf[si + 1], buf[si + 2], buf[si + 3]);
                let di = (dy * LOGO_H + dx) * 4;
                argb[di..di + 4].copy_from_slice(&[a, r, g, b]);
            }
        }
        (LOGO_H, argb)
    })
}

fn put(buf: &mut [u8], w: usize, x: usize, y: usize, argb: [u8; 4]) {
    let i = (y * w + x) * 4;
    if i + 4 <= buf.len() {
        buf[i..i + 4].copy_from_slice(&argb);
    }
}

/// Compose the logo + text into one wide ARGB pixmap for the tray.
fn compose(text: &str) -> Icon {
    let (logo_w, logo_px) = logo();
    let logo_w = *logo_w;

    let scaled_w = GLYPH_W * FONT_SCALE;
    let scaled_h = GLYPH_H * FONT_SCALE;
    // Advance = glyph width + 1 scaled column of spacing.
    let advance = scaled_w + FONT_SCALE;
    let text_w = if text.is_empty() {
        0
    } else {
        text.chars().count() * advance
    };

    let width = logo_w + if text_w > 0 { GAP + text_w } else { 0 };
    let height = PANEL_H;
    let mut data = vec![0u8; width * height * 4];

    // Blit the logo, vertically centered.
    let logo_y0 = (height - logo_w) / 2;
    for ly in 0..logo_w {
        for lx in 0..logo_w {
            let si = (ly * logo_w + lx) * 4;
            let px = [logo_px[si], logo_px[si + 1], logo_px[si + 2], logo_px[si + 3]];
            if px[0] != 0 {
                put(&mut data, width, lx, logo_y0 + ly, px);
            }
        }
    }

    if !text.is_empty() {
        // Build a lit-pixel mask for the whole text at scaled resolution, then
        // paint white where lit and a dark outline on the 4-neighbors of lit
        // pixels — legible on any panel background.
        let text_x0 = logo_w + GAP;
        let text_y0 = (height - scaled_h) / 2;

        let mask_w = text_w;
        let mut mask = vec![false; mask_w * scaled_h];
        for (ci, ch) in text.chars().enumerate() {
            let Some(rows) = glyph(ch) else { continue };
            let gx0 = ci * advance;
            for (ry, row) in rows.iter().enumerate() {
                for gx in 0..GLYPH_W {
                    if row & (1 << (GLYPH_W - 1 - gx)) == 0 {
                        continue;
                    }
                    // Paint the FONT_SCALE×FONT_SCALE block for this font pixel.
                    for sy in 0..FONT_SCALE {
                        for sx in 0..FONT_SCALE {
                            let mx = gx0 + gx * FONT_SCALE + sx;
                            let my = ry * FONT_SCALE + sy;
                            mask[my * mask_w + mx] = true;
                        }
                    }
                }
            }
        }

        let white = [255u8, 255, 255, 255];
        let outline = [220u8, 0, 0, 0];
        for my in 0..scaled_h {
            for mx in 0..mask_w {
                let dx = text_x0 + mx;
                let dy = text_y0 + my;
                if mask[my * mask_w + mx] {
                    put(&mut data, width, dx, dy, white);
                } else {
                    let neighbor = (mx > 0 && mask[my * mask_w + mx - 1])
                        || (mx + 1 < mask_w && mask[my * mask_w + mx + 1])
                        || (my > 0 && mask[(my - 1) * mask_w + mx])
                        || (my + 1 < scaled_h && mask[(my + 1) * mask_w + mx]);
                    if neighbor {
                        put(&mut data, width, dx, dy, outline);
                    }
                }
            }
        }
    }

    Icon {
        width: width as i32,
        height: height as i32,
        data,
    }
}

// ── 5×7 bitmap font (only the glyphs the statusLine string uses) ────────────

/// Rows top→bottom; low 5 bits are the columns (MSB = leftmost).
fn glyph(c: char) -> Option<[u8; 7]> {
    let g = match c {
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00110, 0b00001, 0b00001, 0b10001, 0b01110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00110, 0b00110],
        ':' => [0b00000, 0b00110, 0b00110, 0b00000, 0b00110, 0b00110, 0b00000],
        '%' => [0b11001, 0b11010, 0b00100, 0b01011, 0b10011, 0b00000, 0b00000],
        '|' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'h' => [0b10000, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b10001],
        'd' => [0b00001, 0b00001, 0b00001, 0b01111, 0b10001, 0b10001, 0b01111],
        ' ' => [0; 7],
        _ => return None,
    };
    Some(g)
}

// ── SNI item ────────────────────────────────────────────────────────────────

struct ClaudeTray<R: Runtime> {
    text: String,
    app: AppHandle<R>,
}

impl<R: Runtime> Tray for ClaudeTray<R> {
    fn id(&self) -> String {
        "claudeview".into()
    }

    fn title(&self) -> String {
        // Fallback for hosts that render the SNI title as text.
        if self.text.is_empty() {
            "ClaudeView".into()
        } else {
            format!("ClaudeView — {}", self.text)
        }
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![compose(&self.text)]
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: if self.text.is_empty() {
                "ClaudeView".into()
            } else {
                self.text.clone()
            },
            ..Default::default()
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let app = self.app.clone();
        // Window ops must run on the main (tao) thread; ksni calls us on its own.
        let _ = self.app.run_on_main_thread(move || toggle_main_window(&app));
    }
}

pub fn setup<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    let (tx, rx) = mpsc::channel::<()>();
    let _ = NUDGE.set(tx);

    let app_for_thread = app.clone();
    std::thread::Builder::new()
        .name("claudeview-tray".into())
        .spawn(move || {
            let tray = ClaudeTray {
                text: current_text(),
                app: app_for_thread,
            };
            let handle = match tray.spawn() {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("[tray] ksni spawn failed: {e}");
                    return;
                }
            };

            // Re-render only when the text actually changes.
            let mut last = current_text();
            while rx.recv().is_ok() {
                let next = current_text();
                if next != last {
                    last = next.clone();
                    handle.update(|t: &mut ClaudeTray<R>| t.text = next.clone());
                }
            }
        })
        .expect("spawn claudeview-tray thread");

    Ok(())
}

fn toggle_main_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    match window.is_visible() {
        Ok(true) => {
            let _ = window.hide();
        }
        _ => {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_terminal_style_text() {
        assert_eq!(text_for(Some(48.0), Some(57.0)), "5h: 48.0% | 7d: 57.0%");
        assert_eq!(text_for(Some(48.0), None), "5h: 48.0%");
        assert_eq!(text_for(None, Some(57.0)), "7d: 57.0%");
        assert_eq!(text_for(None, None), "");
    }

    #[test]
    fn composes_nonempty_pixmap() {
        let icon = compose("5h: 48.0% | 7d: 57.0%");
        assert_eq!(icon.height, PANEL_H as i32);
        assert!(icon.width > LOGO_H as i32); // logo + text
        assert_eq!(icon.data.len(), (icon.width * icon.height * 4) as usize);
    }

    #[test]
    fn every_glyph_in_the_format_is_defined() {
        for ch in "5h: 0123456789.% | 7d".chars() {
            assert!(glyph(ch).is_some(), "missing glyph for {ch:?}");
        }
    }
}
