mod app;
mod quota;

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<app::ClaudeViewApplet>(())
}
