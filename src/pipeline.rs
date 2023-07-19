use bevy::{
    pbr::{MeshPipeline, MeshPipelineKey},
    prelude::*,
    render::{
        extract_component::ComponentUniforms,
        mesh::MeshVertexBufferLayout,
        render_resource::{
            BindGroupDescriptor, BindGroupLayout, BindGroupLayoutDescriptor, BindingType,
            BlendComponent, BlendState, BufferBindingType, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, CompareFunction, DepthBiasState, DepthStencilState,
            PipelineCache, RenderPipelineDescriptor, ShaderDefVal, ShaderStages, ShaderType,
            SpecializedMeshPipeline, SpecializedMeshPipelineError, StencilFaceState, StencilState,
            StorageBuffer, TextureFormat,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::BevyDefault,
    },
};

use crate::{
    bind_group_entries, bind_group_layout_entries, utils::RenderPipelineDescriptorBuilder,
    OitLayerIdsBindGroup, OitLayersBindGroup, OitMaterialUniform, OitMaterialUniformsBindGroup,
    OIT_DRAW_SHADER_HANDLE, OIT_LAYERS, OIT_RENDER_SHADER_HANDLE, WINDOW_HEIGHT, WINDOW_WIDTH,
};

#[derive(Resource)]
pub struct OitDrawPipeline {
    pub(crate) mesh_pipeline: MeshPipeline,
    pub(crate) material_bind_group_layout: BindGroupLayout,
    pub(crate) oit_layers_bind_group_layout: BindGroupLayout,
    pub(crate) oit_layer_ids_bind_group_layout: BindGroupLayout,
    pub(crate) oit_layers_buffer: StorageBuffer<Vec<Vec4>>,
    pub(crate) oit_layer_ids_buffer: StorageBuffer<Vec<i32>>,
}

// #[derive(AsBindGroup)]
// struct OitMaterialBindGroup {
//     #[uniform(0)]
//     buffer: OitMaterialUniform,
// }

impl FromWorld for OitDrawPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();

        // let material_bind_group_layout = OitMaterialBindGroup::bind_group_layout(render_device);

        let material_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("oit_material_bind_group_layout"),
                entries: &[bevy::render::render_resource::BindGroupLayoutEntry {
                    binding: 0,
                    ty: (BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: Some(OitMaterialUniform::min_size()),
                    }),
                    visibility: (ShaderStages::FRAGMENT),
                    count: None,
                }],
            });

        let oit_layers_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("oit_layers_bind_group_layout"),
                entries: &bind_group_layout_entries![
                    0 => (ShaderStages::FRAGMENT, BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    }),
                ],
            });

        let oit_layer_ids_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("oit_layers_bind_group_layout"),
                entries: &bind_group_layout_entries![
                    0 => (ShaderStages::FRAGMENT, BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    }),
                ],
            });

        let mesh_pipeline = world.resource::<MeshPipeline>().clone();

        let mut oit_layers_buffer = StorageBuffer::default();
        oit_layers_buffer.set(vec![Vec4::ZERO; WINDOW_WIDTH * WINDOW_HEIGHT * OIT_LAYERS]);
        oit_layers_buffer.write_buffer(render_device, render_queue);

        let mut oit_layer_ids_buffer = StorageBuffer::default();
        oit_layer_ids_buffer.set(vec![0; WINDOW_WIDTH * WINDOW_HEIGHT]);
        oit_layer_ids_buffer.write_buffer(render_device, render_queue);

        OitDrawPipeline {
            mesh_pipeline,
            material_bind_group_layout,
            oit_layers_bind_group_layout,
            oit_layer_ids_bind_group_layout,
            oit_layers_buffer,
            oit_layer_ids_buffer,
        }
    }
}

impl SpecializedMeshPipeline for OitDrawPipeline {
    type Key = MeshPipelineKey;
    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut desc = self.mesh_pipeline.specialize(key, layout)?;

        desc.label = Some("oit_draw_mesh_pipeline".into());

        let mut layout = match key.msaa_samples() {
            1 => vec![self.mesh_pipeline.view_layout.clone()],
            _ => vec![self.mesh_pipeline.view_layout_multisampled.clone()],
        };
        layout.push(self.material_bind_group_layout.clone());
        layout.push(self.mesh_pipeline.mesh_layouts.model_only.clone());
        layout.push(self.oit_layers_bind_group_layout.clone());
        layout.push(self.oit_layer_ids_bind_group_layout.clone());

        let oit_layer_def = ShaderDefVal::Int("OIT_LAYERS".to_string(), OIT_LAYERS as i32);

        desc.layout = layout;
        desc.vertex.shader = OIT_DRAW_SHADER_HANDLE.typed();
        desc.vertex.shader_defs.push(oit_layer_def.clone());
        if let Some(frag) = desc.fragment.as_mut() {
            frag.shader = OIT_DRAW_SHADER_HANDLE.typed();
            frag.shader_defs.push(oit_layer_def);
        }

        Ok(desc)
    }
}

pub(crate) fn queue_bind_group(
    mut commands: Commands,
    pipeline: Res<OitDrawPipeline>,
    render_device: Res<RenderDevice>,
    material_uniforms: Res<ComponentUniforms<OitMaterialUniform>>,
) {
    if let Some(material_uniforms) = material_uniforms.binding() {
        let material_uniforms_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("oit_material_bind_group"),
            layout: &pipeline.material_bind_group_layout,
            entries: &bind_group_entries![
                0 => material_uniforms,
            ],
        });
        commands.insert_resource(OitMaterialUniformsBindGroup(material_uniforms_bind_group));
    }

    if let Some(buffer) = pipeline.oit_layers_buffer.binding() {
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("oit_layers_bind_group"),
            layout: &pipeline.oit_layers_bind_group_layout,
            entries: &bind_group_entries![
                0 => buffer,
            ],
        });
        commands.insert_resource(OitLayersBindGroup(bind_group));
    }

    if let Some(buffer) = pipeline.oit_layer_ids_buffer.binding() {
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("oit_layer_ids_bind_group_layout"),
            layout: &pipeline.oit_layer_ids_bind_group_layout,
            entries: &bind_group_entries![
                0 => buffer,
            ],
        });
        commands.insert_resource(OitLayerIdsBindGroup(bind_group));
    }
}

#[derive(Resource, Deref)]
pub(crate) struct OitRenderPipeline(pub(crate) CachedRenderPipelineId);

pub(crate) fn queue_render_oit_pipeline(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<OitDrawPipeline>,
) {
    let oit_layer_def = ShaderDefVal::Int("OIT_LAYERS".to_string(), OIT_LAYERS as i32);
    let desc = RenderPipelineDescriptorBuilder::fullscreen()
        .label("render_oit_pipeline")
        .depth_stencil(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: false,
            depth_compare: CompareFunction::GreaterEqual,
            stencil: StencilState {
                front: StencilFaceState::IGNORE,
                back: StencilFaceState::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
            bias: DepthBiasState {
                constant: 0,
                slope_scale: 0.0,
                clamp: 0.0,
            },
        })
        .fragment(
            OIT_RENDER_SHADER_HANDLE.typed(),
            "fragment",
            &[ColorTargetState {
                format: TextureFormat::bevy_default(),
                blend: Some(BlendState {
                    color: BlendComponent::OVER,
                    alpha: BlendComponent::OVER,
                }),
                // blend: None,
                write_mask: ColorWrites::ALL,
            }],
            &[oit_layer_def],
        )
        .layout(vec![
            pipeline.oit_layers_bind_group_layout.clone(),
            pipeline.oit_layer_ids_bind_group_layout.clone(),
        ])
        .build();

    let pipeline_id = pipeline_cache.queue_render_pipeline(desc);
    commands.insert_resource(OitRenderPipeline(pipeline_id));
}
