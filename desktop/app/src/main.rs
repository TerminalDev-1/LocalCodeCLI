mod agent_bridge;
mod app_state;
mod setup;
mod theme;
mod view;
mod workspace;

use app_state::State;

fn app_theme(_state: &State) -> iced::Theme {
    theme::theme()
}

fn main() -> iced::Result {
    theme::init();
    iced::application(State::new, State::update, view::view).title(State::title).theme(app_theme).run()
}
