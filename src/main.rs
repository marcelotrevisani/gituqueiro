mod app;
mod config;
mod github;

use app::Gituqueiro;

fn main() -> iced::Result {
    let settings = iced::window::Settings {
        icon: load_icon(),
        ..Default::default()
    };

    iced::application(
        "Gituqueiro - GitHub PR Monitor",
        Gituqueiro::update,
        Gituqueiro::view,
    )
    .subscription(Gituqueiro::subscription)
    .theme(Gituqueiro::theme)
    .window(settings)
    .run_with(Gituqueiro::new)
}

fn load_icon() -> Option<iced::window::Icon> {
    let bytes = include_bytes!("../icons/icon-256x256.png");
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    iced::window::icon::from_rgba(img.into_raw(), w, h).ok()
}
