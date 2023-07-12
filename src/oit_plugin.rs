use bevy::{
    core_pipeline::core_3d,
    prelude::*,
    render::{
        extract_component::{ExtractComponentPlugin, UniformComponentPlugin},
        render_graph::{RenderGraphApp, ViewNodeRunner},
        RenderApp,
    },
};

use crate::{
    clear_pass::{self, ClearNode, ClearPipeline, ClearSettings},
    oit_node::OitNode,
    oit_phase::{OitMaterial, OitSettings},
    post_process_pass::{self, PostProcessNode, PostProcessPipeline, PostProcessSettings},
};

pub const CLEAR_PASS: &str = "clear_pass";
pub const OIT_PASS: &str = "oit_pass";
pub const POST_PROCESS_PASS: &str = "post_process_pass";

pub struct OitPlugin;
impl Plugin for OitPlugin {
    fn build(&self, app: &mut App) {
        app
            // clear pass
            .add_plugins((
                ExtractComponentPlugin::<ClearSettings>::default(),
                UniformComponentPlugin::<ClearSettings>::default(),
            ))
            .add_systems(Update, clear_pass::update_settings)
            // oit phase
            .add_plugins((
                ExtractComponentPlugin::<OitMaterial>::default(),
                UniformComponentPlugin::<OitMaterial>::default(),
                ExtractComponentPlugin::<OitSettings>::default(),
                UniformComponentPlugin::<OitSettings>::default(),
            ))
            // post process
            .add_plugins((
                ExtractComponentPlugin::<PostProcessSettings>::default(),
                UniformComponentPlugin::<PostProcessSettings>::default(),
            ))
            .add_systems(Update, post_process_pass::update_settings);

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        const CORE_3D: &str = core_3d::graph::NAME;

        use core_3d::graph::node::*;
        render_app
            .add_render_graph_node::<ViewNodeRunner<ClearNode>>(CORE_3D, CLEAR_PASS)
            .add_render_graph_node::<ViewNodeRunner<OitNode>>(CORE_3D, OIT_PASS)
            .add_render_graph_node::<ViewNodeRunner<PostProcessNode>>(CORE_3D, POST_PROCESS_PASS)
            .add_render_graph_edges(CORE_3D, &[END_MAIN_PASS, OIT_PASS, POST_PROCESS_PASS])
            .add_render_graph_edges(CORE_3D, &[CLEAR_PASS, OIT_PASS]);
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<PostProcessPipeline>()
            .init_resource::<ClearPipeline>();
    }
}
