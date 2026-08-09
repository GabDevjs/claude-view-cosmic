// A COSMIC panel applet that renders the local machine's 5-hour Claude Code
// usage as the Claude logo + the bare percentage (e.g. `94%`) in the panel.
// When the window hits 100% the panel also shows when it resets. Clicking opens
// a popup with the reset time and countdown. This is the only way to get real
// inline text on the COSMIC panel; the SNI tray only allows a square icon.

use chrono::{Local, TimeZone};
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::{futures, window::Id, Alignment, Limits, Subscription};
use cosmic::prelude::*;
use cosmic::widget;
use futures::SinkExt;
use std::sync::LazyLock;
use std::time::Duration;

use crate::quota::{self, Quota};

/// Stable id for the autosizing panel button (so it grows/shrinks to the text).
static AUTOSIZE_ID: LazyLock<widget::Id> = LazyLock::new(|| widget::Id::new("claudeview-applet"));

const REFRESH: Duration = Duration::from_secs(5);

/// Monochrome Claude logo (single path, `fill="#000000"`) — we swap the fill
/// color per render so the icon follows the theme text color normally and turns
/// orange / red as usage climbs.
const CLAUDE_SVG: &str = r##"<svg fill="#000000" fill-rule="evenodd" height="24" viewBox="0 0 24 24" width="24" xmlns="http://www.w3.org/2000/svg"><title>Claude Code</title><path clip-rule="evenodd" d="M20.998 10.949H24v3.102h-3v3.028h-1.487V20H18v-2.921h-1.487V20H15v-2.921H9V20H7.488v-2.921H6V20H4.487v-2.921H3V14.05H0V10.95h3V5h17.998v5.949zM6 10.949h1.488V8.102H6v2.847zm10.51 0H18V8.102h-1.49v2.847z"></path></svg>"##;

/// Reset time as local `HH:MM`.
fn reset_clock(ts: i64) -> String {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|dt| dt.format("%H:%M").to_string())
        .unwrap_or_else(|| "--:--".into())
}

/// Time left until `ts` as `40min` or `1h 40min`; `None` once it's past.
fn countdown(ts: i64) -> Option<String> {
    let secs = ts - Local::now().timestamp();
    if secs <= 0 {
        return None;
    }
    let mins = secs / 60;
    Some(if mins < 60 {
        format!("{mins}min")
    } else {
        let (h, m) = (mins / 60, mins % 60);
        if m == 0 {
            format!("{h}h")
        } else {
            format!("{h}h {m}min")
        }
    })
}

/// Panel text: just the 5h value (`94%`); at 100% it also shows the reset time.
fn panel_text(q: &Quota) -> String {
    let Some(p) = q.five else {
        return "—".to_string();
    };
    let shown = p.round().max(0.0) as i64;
    let base = format!("{shown}%");
    if shown >= 100 {
        if let Some(ts) = q.five_reset {
            let tail = match countdown(ts) {
                Some(cd) => format!(" faltam {cd}"),
                None => String::new(),
            };
            return format!("{base} · reinicia às {}{}", reset_clock(ts), tail);
        }
    }
    base
}

/// Icon color for the given usage: theme text color when calm, orange ≥80%, red ≥95%.
fn usage_rgb(core: &cosmic::Core, q: &Quota) -> [u8; 3] {
    match q.five {
        Some(u) if u >= 95.0 => [0xf4, 0x43, 0x36], // red
        Some(u) if u >= 80.0 => [0xff, 0x98, 0x00], // orange
        _ => {
            let c = core.system_theme().cosmic().on_bg_color();
            [
                (c.red * 255.0).round() as u8,
                (c.green * 255.0).round() as u8,
                (c.blue * 255.0).round() as u8,
            ]
        }
    }
}

/// Build the Claude logo tinted to `rgb`.
fn claude_icon(rgb: [u8; 3]) -> cosmic::widget::icon::Handle {
    let hex = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
    let svg = CLAUDE_SVG.replace("#000000", &hex);
    cosmic::widget::icon::from_svg_bytes(svg.into_bytes())
}

#[derive(Default)]
pub struct ClaudeViewApplet {
    /// State managed by the COSMIC runtime.
    core: cosmic::Core,
    /// Latest usage read from the cache.
    quota: Quota,
    /// The details popup, when open.
    popup: Option<Id>,
}

#[derive(Debug, Clone)]
pub enum Message {
    /// Periodic refresh: re-read the cache.
    Tick,
    /// Panel item clicked: open/close the details popup.
    TogglePopup,
    /// The popup was closed.
    PopupClosed(Id),
    /// Open the full ClaudeView window.
    OpenApp,
}

impl cosmic::Application for ClaudeViewApplet {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "dev.gabdevjs.ClaudeViewApplet";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let app = ClaudeViewApplet {
            core,
            quota: quota::read(),
            popup: None,
        };
        (app, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    /// Refresh every few seconds from the cache (also ticks the countdown).
    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::run(|| {
            cosmic::iced::stream::channel(1, |mut output: futures::channel::mpsc::Sender<_>| async move {
                let mut interval = tokio::time::interval(REFRESH);
                loop {
                    interval.tick().await;
                    let _ = output.send(Message::Tick).await;
                }
            })
        })
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::Tick => {
                self.quota = quota::read();
            }
            Message::OpenApp => {
                let _ = std::process::Command::new("claudeview").spawn();
            }
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(320.0)
                        .min_width(200.0)
                        .min_height(80.0)
                        .max_height(600.0);
                    get_popup(popup_settings)
                };
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
        }
        Task::none()
    }

    /// The panel button: the Claude logo + 5h percentage, autosized and clickable.
    fn view(&self) -> Element<'_, Self::Message> {
        let rgb = usage_rgb(&self.core, &self.quota);
        let icon = widget::icon::icon(claude_icon(rgb)).size(16);
        let text = self.core.applet.text(panel_text(&self.quota));
        let content = widget::row::with_children(vec![icon.into(), text.into()])
            .align_y(Alignment::Center)
            .spacing(6);
        let button = widget::button::custom(content)
            .class(cosmic::theme::Button::AppletIcon)
            .on_press(Message::TogglePopup);
        widget::autosize::autosize(widget::container(button), AUTOSIZE_ID.clone()).into()
    }

    /// The details popup: 5h usage, reset time and countdown.
    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        let mut col = widget::column::with_capacity(4)
            .spacing(6)
            .padding(12)
            .push(widget::text::title4("ClaudeView"));

        match self.quota.five {
            Some(p) => {
                col = col.push(widget::text(format!("Sessão de 5h: {}%", p.round() as i64)));
                if let Some(ts) = self.quota.five_reset {
                    col = col.push(widget::text(format!("Reinicia às {}", reset_clock(ts))));
                    col = col.push(match countdown(ts) {
                        Some(cd) => widget::text(format!("Faltam {cd}")),
                        None => widget::text("Reiniciando…"),
                    });
                }
            }
            None => {
                col = col.push(widget::text("sem dados — verifique o hook do statusLine"));
            }
        }

        col = col.push(
            widget::button::text("Abrir janela do ClaudeView").on_press(Message::OpenApp),
        );

        self.core.applet.popup_container(col).into()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}
