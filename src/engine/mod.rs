mod app;
mod atlas;
mod cam;
mod cross;
#[cfg(feature = "hermit")]
mod mca;
mod pipe;
mod state;
mod texture;
mod uniform;
#[cfg(feature = "hot-reload")]
mod watcher;
mod world;

#[macro_export]
macro_rules! shader_path {
    ($path:expr) => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/", $path)
    };
}

pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<state::State>>,
    state: Option<state::State>,
    last_update: instant::Instant,
}
