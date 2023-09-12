//! A simple material that implements gooch shading. The same shading is used for the OIT material so it makes comparison easier

use bevy::{
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::render_resource::{AsBindGroup, ShaderRef},
};

#[derive(AsBindGroup, TypeUuid, TypePath, Debug, Clone, Default)]
#[uuid = "fd884c25-98b1-5155-a809-881b0740b498"]
pub struct GoochMaterial {
    #[uniform(0)]
    pub base_color: Color,
    pub depth_bias: f32,
}
impl Material for GoochMaterial {
    fn fragment_shader() -> ShaderRef {
        "gooch_shading.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    fn depth_bias(&self) -> f32 {
        self.depth_bias
    }
}
