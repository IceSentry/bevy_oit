use bevy::{
    pbr::{MeshPipeline, MeshPipelineKey},
    prelude::*,
    render::{
        extract_component::ComponentUniforms,
        mesh::MeshVertexBufferLayout,
        render_resource::{
            BindGroupDescriptor, BindGroupLayout, BindGroupLayoutDescriptor, BindingType,
            BufferBindingType, CachedRenderPipelineId, ColorTargetState, ColorWrites,
            PipelineCache, RenderPipelineDescriptor, ShaderDefVal, ShaderStages, ShaderType,
            SpecializedMeshPipeline, SpecializedMeshPipelineError, StorageBuffer, TextureFormat,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::BevyDefault,
    },
};

use crate::{
    bind_group_entries, bind_group_layout_entries,
    oit_plugin::{
        OitLayerIdsBindGroup, OitLayersBindGroup, OitMaterialBindGroup, OitMaterialUniform,
        OIT_DRAW_SHADER_HANDLE, OIT_RENDER_SHADER_HANDLE,
    },
    utils::RenderPipelineDescriptorBuilder,
    OIT_LAYERS, WINDOW_HEIGHT, WINDOW_WIDTH,
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

impl FromWorld for OitDrawPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();

        let material_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("oit_bind_group_layout"),
                entries: &bind_group_layout_entries![
                    0 => (ShaderStages::FRAGMENT, BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: Some(OitMaterialUniform::min_size()),
                    }),
                ],
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
        desc.fragment.as_mut().unwrap().shader = OIT_DRAW_SHADER_HANDLE.typed();
        desc.fragment
            .as_mut()
            .unwrap()
            .shader_defs
            .push(oit_layer_def);

        Ok(desc)
    }
}

pub(crate) fn queue_bind_group(
    mut commands: Commands,
    pipeline: Res<OitDrawPipeline>,
    render_device: Res<RenderDevice>,
    material_uniforms: Res<ComponentUniforms<OitMaterialUniform>>,
) {
    if let Some(material_uniform) = material_uniforms.binding() {
        let material_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("oit_material_bind_group"),
            layout: &pipeline.material_bind_group_layout,
            entries: &bind_group_entries![
                0 => material_uniform,
            ],
        });
        commands.insert_resource(OitMaterialBindGroup(material_bind_group));
    }

    if let Some(buffer) = pipeline.oit_layers_buffer.binding() {
        let oit_layers_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("oit_layers_bind_group"),
            layout: &pipeline.oit_layers_bind_group_layout,
            entries: &bind_group_entries![
                0 => buffer,
            ],
        });
        commands.insert_resource(OitLayersBindGroup(oit_layers_bind_group));
    }

    if let Some(buffer) = pipeline.oit_layer_ids_buffer.binding() {
        let oit_layer_ids_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("oit_layer_ids_bind_group_layout"),
            layout: &pipeline.oit_layer_ids_bind_group_layout,
            entries: &bind_group_entries![
                0 => buffer,
            ],
        });
        commands.insert_resource(OitLayerIdsBindGroup(oit_layer_ids_bind_group));
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
    let pipeline_id = pipeline_cache.queue_render_pipeline(
        RenderPipelineDescriptorBuilder::fullscreen()
            .label("render_oit_pipeline")
            .fragment(
                OIT_RENDER_SHADER_HANDLE.typed(),
                "fragment",
                &[ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: None,
                    write_mask: ColorWrites::ALL,
                }],
                &[oit_layer_def],
            )
            .layout(vec![
                pipeline.oit_layers_bind_group_layout.clone(),
                pipeline.oit_layer_ids_bind_group_layout.clone(),
            ])
            .build(),
    );
    commands.insert_resource(OitRenderPipeline(pipeline_id));
}
