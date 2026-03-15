mod app;
mod cam;
mod mca;
mod pipe;
mod state;
mod texture;
mod watcher;
mod world;

pub struct App {
    state: Option<state::State>,
}
