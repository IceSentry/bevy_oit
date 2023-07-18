use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_phase::RenderPhase,
        render_resource::{
            LoadOp, Operations, PipelineCache, RenderPassDepthStencilAttachment,
            RenderPassDescriptor,
        },
        renderer::RenderContext,
        view::{ViewDepthTexture, ViewTarget},
    },
};

use crate::{pipeline::OitRenderPipeline, OitLayerIdsBindGroup, OitLayersBindGroup, OitPhaseItem};

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
        &'static ViewDepthTexture,
    );

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (camera, render_phase, view_target, depth): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.view_entity();

        if render_phase.items.is_empty() {
            return Ok(());
        }

        // oit draw phase
        {
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("oit_draw_pass"),
                color_attachments: &[Some(view_target.get_color_attachment(Operations {
                    load: LoadOp::Load,
                    store: true,
                }))],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            if let Some(viewport) = camera.viewport.as_ref() {
                render_pass.set_camera_viewport(viewport);
            }

            render_phase.render(&mut render_pass, world, view_entity);
        }

        // render oit
        // TODO this should probably run after the main transparent pass
        {
            let pipeline_id = world.resource::<OitRenderPipeline>();
            let pipeline_cache = world.resource::<PipelineCache>();
            let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id.0) else {
                return Ok(());
            };

            let oit_layers_bind_group = world.resource::<OitLayersBindGroup>();
            let oit_layer_ids_bind_group = world.resource::<OitLayerIdsBindGroup>();

            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("oit_render_pass"),
                color_attachments: &[Some(view_target.get_color_attachment(Operations {
                    load: LoadOp::Load,
                    store: true,
                }))],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            render_pass.set_render_pipeline(pipeline);
            render_pass.set_bind_group(0, oit_layers_bind_group, &[]);
            render_pass.set_bind_group(1, oit_layer_ids_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        Ok(())
    }
}
