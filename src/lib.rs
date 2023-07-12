pub const WINDOW_WIDTH: usize = 1280;
pub const WINDOW_HEIGHT: usize = 720;
pub const OIT_LAYERS: usize = 8;

pub mod clear_pass;
pub mod custom_phase;
pub mod oit_node;
pub mod oit_phase;
pub mod oit_plugin;
pub mod post_process_pass;
pub mod render_oit;
mod utils;
