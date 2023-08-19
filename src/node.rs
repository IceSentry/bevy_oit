use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_phase::RenderPhase,
        render_resource::{LoadOp, Operations, PipelineCache, RenderPassDescriptor},
        renderer::RenderContext,
        view::{ViewTarget, ViewUniformOffset},
    },
};

use crate::{
    pipeline::{OitRenderPipelineId, OitRenderViewBindGroup},
    OitLayersBindGroup, OitPhaseItem,
};

#[derive(Default)]
pub struct OitNode;
impl OitNode {
    pub const NAME: &str = "oit_node";
}

impl ViewNode for OitNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static RenderPhase<OitPhaseItem>,
        &'static ViewTarget,
        &'static OitLayersBindGroup,
        &'static ViewUniformOffset,
    );

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (camera, render_phase, view_target, oit_layers_bind_group, view_uniform): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.view_entity();

        if render_phase.items.is_empty() {
            return Ok(());
        }

        // draw oit phase
        {
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("oit_draw_pass"),
                color_attachments: &[Some(view_target.get_color_attachment(Operations {
                    load: LoadOp::Load,
                    store: true,
                }))],
                depth_stencil_attachment: None,
            });

            if let Some(viewport) = camera.viewport.as_ref() {
                render_pass.set_camera_viewport(viewport);
            }

            render_phase.render(&mut render_pass, world, view_entity);
        }

        // render oit
        {
            let pipeline_id = world.resource::<OitRenderPipelineId>();
            let pipeline_cache = world.resource::<PipelineCache>();
            let render_view_bind_group = world.resource::<OitRenderViewBindGroup>();
            let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id.0) else {
                return Ok(());
            };

            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("oit_render_pass"),
                color_attachments: &[Some(view_target.get_color_attachment(Operations {
                    load: LoadOp::Load,
                    store: true,
                }))],
                depth_stencil_attachment: None,
            });

            render_pass.set_render_pipeline(pipeline);
            render_pass.set_bind_group(0, render_view_bind_group, &[view_uniform.offset]);
            render_pass.set_bind_group(1, oit_layers_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        Ok(())
    }
}
