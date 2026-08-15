mod agent_bridge;
mod app_state;
mod setup;
mod view;

use app_state::State;

fn main() -> iced::Result {
    iced::application(State::new, State::update, view::view).title(State::title).run()
}
