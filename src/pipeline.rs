use bevy::{
    pbr::{MeshPipeline, MeshPipelineKey},
    prelude::*,
    render::{
        extract_component::ComponentUniforms,
        mesh::MeshVertexBufferLayout,
        render_resource::{
            BindGroup, BindGroupLayout, BindGroupLayoutDescriptor, BindingType, BlendComponent,
            BlendState, BufferBindingType, CachedRenderPipelineId, ColorTargetState, ColorWrites,
            PipelineCache, RenderPipelineDescriptor, ShaderDefVal, ShaderStages, ShaderType,
            SpecializedMeshPipeline, SpecializedMeshPipelineError, StorageBuffer, TextureFormat,
            TextureSampleType, TextureViewDimension,
        },
        renderer::RenderDevice,
        texture::BevyDefault,
        view::{ViewDepthTexture, ViewUniform, ViewUniforms},
    },
    utils::HashMap,
};

use crate::{
    utils::{BindingResouceExt, RenderDeviceExt, RenderPipelineDescriptorBuilder},
    OitDepthBindGroup, OitLayerIdsBindGroup, OitLayersBindGroup, OitMaterialUniform,
    OitMaterialUniformsBindGroup, OIT_DRAW_SHADER_HANDLE, OIT_LAYERS, OIT_RENDER_SHADER_HANDLE,
};

#[derive(Resource)]
pub struct OitDrawPipeline {
    pub(crate) mesh_pipeline: MeshPipeline,
    pub(crate) oit_material_bind_group_layout: BindGroupLayout,
    pub(crate) oit_layers_bind_group_layout: BindGroupLayout,
    pub(crate) oit_layer_ids_bind_group_layout: BindGroupLayout,
    pub(crate) depth_bind_group_layout: BindGroupLayout,
}

impl FromWorld for OitDrawPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let oit_material_bind_group_layout = render_device.create_bind_group_layout_ext(
            "oit_material_bind_group_layout",
            [(
                ShaderStages::FRAGMENT,
                BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(OitMaterialUniform::min_size()),
                },
            )],
        );

        let oit_layers_bind_group_layout = render_device.create_bind_group_layout_ext(
            "oit_layers_bind_group_layout",
            [(
                ShaderStages::FRAGMENT,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            )],
        );

        let oit_layer_ids_bind_group_layout = render_device.create_bind_group_layout_ext(
            "oit_layer_ids_bind_group_layout",
            [(
                ShaderStages::FRAGMENT,
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            )],
        );

        let depth_bind_group_layout = render_device.create_bind_group_layout_ext(
            "depth_bind_group_layout",
            [(
                ShaderStages::FRAGMENT,
                BindingType::Texture {
                    sample_type: TextureSampleType::Depth,
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
            )],
        );

        let mesh_pipeline = world.resource::<MeshPipeline>().clone();

        OitDrawPipeline {
            mesh_pipeline,
            oit_material_bind_group_layout,
            oit_layers_bind_group_layout,
            oit_layer_ids_bind_group_layout,
            depth_bind_group_layout,
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
        layout.push(self.oit_material_bind_group_layout.clone());
        layout.push(self.mesh_pipeline.mesh_layouts.model_only.clone());
        layout.push(self.oit_layers_bind_group_layout.clone());
        layout.push(self.oit_layer_ids_bind_group_layout.clone());
        layout.push(self.depth_bind_group_layout.clone());

        let oit_layer_def = ShaderDefVal::Int("OIT_LAYERS".to_string(), OIT_LAYERS as i32);

        desc.layout = layout;
        desc.vertex.shader = OIT_DRAW_SHADER_HANDLE.typed();
        desc.vertex.shader_defs.push(oit_layer_def.clone());
        if let Some(frag) = desc.fragment.as_mut() {
            frag.shader = OIT_DRAW_SHADER_HANDLE.typed();
            frag.shader_defs.push(oit_layer_def);
        }
        desc.depth_stencil = None;

        Ok(desc)
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
#[allow(clippy::type_complexity)]
pub struct OitBuffers(
    pub HashMap<Entity, (usize, StorageBuffer<Vec<UVec2>>, StorageBuffer<Vec<i32>>)>,
);

#[derive(Resource, Deref)]
pub struct OitRenderParamsBindGroup(pub BindGroup);

#[allow(clippy::too_many_arguments)]
pub fn queue_bind_groups(
    mut commands: Commands,
    pipeline: Res<OitDrawPipeline>,
    render_pipeline: Res<OitRenderPipeline>,
    render_device: Res<RenderDevice>,
    material_uniforms: Res<ComponentUniforms<OitMaterialUniform>>,
    buffers: Res<OitBuffers>,
    view_uniforms: Res<ViewUniforms>,
    depth_textures: Query<(Entity, &ViewDepthTexture)>,
) {
    let material_uniforms_bind_group = render_device.create_bind_group_ext(
        "oit_material_bind_group",
        &pipeline.oit_material_bind_group_layout,
        [material_uniforms.uniforms().binding_entry()],
    );
    commands.insert_resource(OitMaterialUniformsBindGroup(material_uniforms_bind_group));

    for (entity, (_, oit_layers_buffer, oit_layer_ids_buffer)) in &buffers.0 {
        let bind_group = render_device.create_bind_group_ext(
            "oit_layers_bind_group",
            &pipeline.oit_layers_bind_group_layout,
            [oit_layers_buffer.binding_entry()],
        );
        commands
            .entity(*entity)
            .insert(OitLayersBindGroup(bind_group));

        let bind_group = render_device.create_bind_group_ext(
            "oit_layer_ids_bind_group",
            &pipeline.oit_layer_ids_bind_group_layout,
            [oit_layer_ids_buffer.binding_entry()],
        );
        commands
            .entity(*entity)
            .insert(OitLayerIdsBindGroup(bind_group));
    }

    let bind_group = render_device.create_bind_group_ext(
        "oit_render_params_bind_gropu",
        &render_pipeline.params_bind_group_layout,
        [view_uniforms.uniforms.binding_entry()],
    );
    commands.insert_resource(OitRenderParamsBindGroup(bind_group));

    for (e, texture) in &depth_textures {
        let bind_group = render_device.create_bind_group_ext(
            "oit_draw_depth_bind_group",
            &pipeline.depth_bind_group_layout,
            [texture.view.binding_entry()],
        );
        commands.entity(e).insert(OitDepthBindGroup(bind_group));
    }
}

#[derive(Resource)]
pub struct OitRenderPipeline {
    params_bind_group_layout: BindGroupLayout,
}

impl FromWorld for OitRenderPipeline {
    fn from_world(world: &mut World) -> Self {
        let params_bind_group_layout = world
            .resource::<RenderDevice>()
            .create_bind_group_layout_ext(
                "oit_render_params_layout",
                [(
                    ShaderStages::FRAGMENT,
                    BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: Some(ViewUniform::min_size()),
                    },
                )],
            );
        OitRenderPipeline {
            params_bind_group_layout,
        }
    }
}

#[derive(Resource, Deref)]
pub struct OitRenderPipelineId(pub CachedRenderPipelineId);

pub fn queue_render_oit_pipeline(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    draw_pipeline: Res<OitDrawPipeline>,
    render_pipeline: Res<OitRenderPipeline>,
) {
    let oit_layer_def = ShaderDefVal::Int("OIT_LAYERS".to_string(), OIT_LAYERS as i32);
    let desc = RenderPipelineDescriptorBuilder::fullscreen()
        .label("render_oit_pipeline")
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
            draw_pipeline.oit_layers_bind_group_layout.clone(),
            draw_pipeline.oit_layer_ids_bind_group_layout.clone(),
            render_pipeline.params_bind_group_layout.clone(),
        ])
        .build();

    let pipeline_id = pipeline_cache.queue_render_pipeline(desc);
    commands.insert_resource(OitRenderPipelineId(pipeline_id));
}
