use bevy::{
    prelude::{QueryState, World},
    render::{
        camera::ExtractedCamera,
        render_graph::{Node, RenderGraphContext, SlotInfo, SlotType},
        render_phase::RenderPhase,
        render_resource::{
            LoadOp, Operations, RenderPassDepthStencilAttachment, RenderPassDescriptor,
        },
        renderer::RenderContext,
        view::{ViewDepthTexture, ViewTarget},
    },
};

use crate::oit_phase::Oit;

pub struct OitNode {
    query: QueryState<(
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static ViewDepthTexture,
        &'static RenderPhase<Oit>,
    )>,
}

impl OitNode {
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for OitNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(Self::IN_VIEW, SlotType::Entity)]
    }

    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let Ok((
            camera,
            view_target,
            depth,
            phase,
        )) = self.query.get_manual(world, view_entity) else {
            return Ok(());
        };

        if phase.items.is_empty() {
            return Ok(());
        }

        let mut pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("oit_pass"),
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
            pass.set_camera_viewport(viewport);
        }

        phase.render(&mut pass, world, view_entity);

        Ok(())
    }
}
