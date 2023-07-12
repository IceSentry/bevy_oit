use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_phase::RenderPhase,
        render_resource::{
            BindGroupDescriptor, LoadOp, Operations, RenderPassDepthStencilAttachment,
            RenderPassDescriptor,
        },
        renderer::RenderContext,
        view::{ViewDepthTexture, ViewTarget},
    },
};

use crate::bind_group_entries;

use super::{pipeline::CustomPipeline, CustomPhaseItem};

#[derive(Default)]
pub struct CustomNode;
impl CustomNode {
    pub const NAME: &str = "custom_node";
}

impl ViewNode for CustomNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static RenderPhase<CustomPhaseItem>,
        &'static ViewTarget,
        &'static ViewDepthTexture,
    );

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (camera, render_phase, target, depth): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.view_entity();

        if render_phase.items.is_empty() {
            return Ok(());
        }

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("custom_pass"),
            color_attachments: &[Some(target.get_color_attachment(Operations {
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

        let pipeline = world.resource::<CustomPipeline>();
        let bind_group = render_context
            .render_device()
            .create_bind_group(&BindGroupDescriptor {
                label: Some("post_process_bind_group"),
                layout: &pipeline.oit_layers_bind_group_layout,
                entries: &bind_group_entries![
                    0 => pipeline.oit_layers_buffer.binding().unwrap(),
                ],
            });

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("post_process_pass"),
            color_attachments: &[Some(target.get_color_attachment(Operations {
                load: LoadOp::Load,
                store: true,
            }))],
            depth_stencil_attachment: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}
