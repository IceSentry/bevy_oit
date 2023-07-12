use crate::{
    bind_group_entries, bind_group_layout_entries,
    oit_phase::OitPipeline,
    utils::{color_target, RenderPipelineDescriptorBuilder},
};
use bevy::render::render_resource::{
    BindingType, BufferBindingType, Operations, RenderPassColorAttachment, ShaderStages,
};
use bevy::render::view::ViewTarget;
use bevy::{ecs::query::QueryItem, render::render_graph::ViewNode};
use bevy::{
    prelude::*,
    render::{
        extract_component::{ComponentUniforms, ExtractComponent},
        render_graph::{NodeRunError, RenderGraphContext},
        render_resource::{
            BindGroupDescriptor, BindGroupLayout, BindGroupLayoutDescriptor,
            CachedRenderPipelineId, PipelineCache, RenderPassDescriptor, Sampler,
            SamplerDescriptor, ShaderType,
        },
        renderer::{RenderContext, RenderDevice},
    },
};

#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType)]
pub struct ClearSettings {
    pub viewport_width: f32,
}

#[derive(Default)]
pub struct ClearNode;
impl ViewNode for ClearNode {
    type ViewQuery = &'static ViewTarget;
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        view_target: QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let clear_pipeline = world.resource::<ClearPipeline>();
        let oit_pipeline = world.resource::<OitPipeline>();

        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(clear_pipeline.pipeline_id) else {
            return Ok(());
        };

        let settings_uniforms = world.resource::<ComponentUniforms<ClearSettings>>();
        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();

        // TODO can be done in Queue phase
        let bind_group = render_context
            .render_device()
            .create_bind_group(&BindGroupDescriptor {
                label: Some("clear_bind_group"),
                layout: &clear_pipeline.layout,
                entries: &bind_group_entries![
                    0 => settings_binding.clone(),
                    1 => oit_pipeline.counter_buffer.binding().unwrap(),
                ],
            });

        let layers_bind_group =
            render_context
                .render_device()
                .create_bind_group(&BindGroupDescriptor {
                    label: Some("clear_layers_bind_group"),
                    layout: &clear_pipeline.layout,
                    entries: &bind_group_entries![
                        0 => oit_pipeline.layers.binding().unwrap(),
                    ],
                });

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("clear_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.set_bind_group(1, &layers_bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

#[derive(Resource)]
pub struct ClearPipeline {
    pub layout: BindGroupLayout,
    pub layers_layout: BindGroupLayout,
    pub sampler: Sampler,
    pub pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for ClearPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("clear_bind_group_layout"),
            entries: &bind_group_layout_entries![
                // settings
                0 => (ShaderStages::VERTEX_FRAGMENT, BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                }),
                // counter
                1 => (ShaderStages::VERTEX_FRAGMENT, BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                }),
            ],
        });

        let layers_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("clear_bind_group_layers_layout"),
            entries: &bind_group_layout_entries![
                // oit layers buffer
                0 => (ShaderStages::VERTEX_FRAGMENT, BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                }),
            ],
        });

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        let shader = world.resource::<AssetServer>().load("clear.wgsl");

        let descriptor = RenderPipelineDescriptorBuilder::fullscreen()
            .label("clear_pipeline")
            .layout(vec![layout.clone(), layers_layout.clone()])
            .fragment(shader, "fragment", &[color_target(None)], &[])
            .build();

        let pipeline_id = world
            .resource_mut::<PipelineCache>()
            .queue_render_pipeline(descriptor);

        Self {
            layout,
            layers_layout,
            sampler,
            pipeline_id,
        }
    }
}

pub fn update_settings(mut q: Query<(&mut ClearSettings, &Camera)>) {
    for (mut settings, camera) in &mut q {
        settings.viewport_width = camera.physical_viewport_size().unwrap().x as f32;
        // settings.viewport_height = camera.physical_viewport_size().unwrap().y as f32;
    }
}
