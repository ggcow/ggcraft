mod app;
mod atlas;
mod cam;
#[cfg(feature = "hermit")]
mod mca;
mod pipe;
mod state;
mod texture;
#[cfg(feature = "hot-reload")]
mod watcher;
mod world;

#[macro_export]
macro_rules! SHADER_PATH {
    () => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/block.wgsl")
    };
}

pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<state::State>>,
    state: Option<state::State>,
    last_update: instant::Instant,
}
