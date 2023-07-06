use crate::{
    bind_group_entries, bind_group_layout_entries,
    oit_phase::OitPipeline,
    utils::{color_target, vertex_state, RenderPipelineDescriptorBuilder},
};
use bevy::render::render_resource::{
    BindingResource, BindingType, BufferBindingType, SamplerBindingType, ShaderStages,
    TextureSampleType, TextureViewDimension,
};
use bevy::{ecs::query::QueryItem, render::render_graph::ViewNode};
use bevy::{
    prelude::*,
    render::{
        extract_component::{ComponentUniforms, ExtractComponent},
        render_graph::{NodeRunError, RenderGraphContext},
        render_resource::{
            BindGroupDescriptor, BindGroupLayout, BindGroupLayoutDescriptor,
            CachedRenderPipelineId, Operations, PipelineCache, RenderPassColorAttachment,
            RenderPassDescriptor, Sampler, SamplerDescriptor, ShaderType,
        },
        renderer::{RenderContext, RenderDevice},
        view::ViewTarget,
    },
};

#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType)]
pub struct PostProcessSettings {
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub oit_layers: u32,
}

#[derive(Default)]
pub struct PostProcessNode;
impl ViewNode for PostProcessNode {
    type ViewQuery = &'static ViewTarget;
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        view_target: QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let post_process_pipeline = world.resource::<PostProcessPipeline>();
        let oit_pipeline = world.resource::<OitPipeline>();

        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(post_process_pipeline.pipeline_id) else {
            return Ok(());
        };

        let settings_uniforms = world.resource::<ComponentUniforms<PostProcessSettings>>();
        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();

        let bind_group = render_context
            .render_device()
            .create_bind_group(&BindGroupDescriptor {
                label: Some("post_process_bind_group"),
                layout: &post_process_pipeline.layout,
                entries: &bind_group_entries![
                    0 => BindingResource::TextureView(post_process.source),
                    1 => BindingResource::Sampler(&post_process_pipeline.sampler),
                    2 => settings_binding.clone(),
                    3 => oit_pipeline.counter_buffer.binding().unwrap(),
                    4 => oit_pipeline.layers.binding().unwrap(),
                ],
            });

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("post_process_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

#[derive(Resource)]
pub struct PostProcessPipeline {
    pub layout: BindGroupLayout,
    pub sampler: Sampler,
    pub pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for PostProcessPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("post_process_bind_group_layout"),
            entries: &bind_group_layout_entries![
                // texture
                0 => (ShaderStages::FRAGMENT, BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                }),
                // sampler
                1 => (ShaderStages::FRAGMENT, BindingType::Sampler(
                    SamplerBindingType::Filtering
                )),
                // settings
                2 => (ShaderStages::FRAGMENT, BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                }),
                // counter
                3 => (ShaderStages::FRAGMENT, BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                }),
                // oit layers buffer
                4 => (ShaderStages::FRAGMENT, BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                }),
            ],
        });

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        let shader = world
            .resource::<AssetServer>()
            .load("post_process_pass.wgsl");

        let descriptor =
            RenderPipelineDescriptorBuilder::new(vertex_state(shader.clone(), "vertex", &[], &[]))
                .label("post_process_pipeline")
                .layout(vec![layout.clone()])
                .fragment(shader, "fragment", &[color_target(None)], &[])
                .build();

        let pipeline_id = world
            .resource_mut::<PipelineCache>()
            .queue_render_pipeline(descriptor);

        Self {
            layout,
            sampler,
            pipeline_id,
        }
    }
}

pub fn update_settings(mut q: Query<(&mut PostProcessSettings, &Camera)>) {
    for (mut settings, camera) in &mut q {
        settings.viewport_width = camera.physical_viewport_size().unwrap().x as f32;
        settings.viewport_height = camera.physical_viewport_size().unwrap().y as f32;
    }
}
