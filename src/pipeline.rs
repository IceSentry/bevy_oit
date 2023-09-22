use bevy::{
    pbr::{MeshPipeline, MeshPipelineKey},
    prelude::*,
    render::{
        mesh::MeshVertexBufferLayout,
        render_resource::{
            AsBindGroup, BindGroup, BindGroupLayout, BlendComponent, BlendState,
            CachedRenderPipelineId, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState,
            DepthStencilState, MultisampleState, PipelineCache, RenderPipelineDescriptor,
            ShaderDefVal, ShaderStages, ShaderType, SpecializedMeshPipeline,
            SpecializedMeshPipelineError, StencilState, StorageBuffer, TextureFormat,
        },
        renderer::RenderDevice,
        texture::BevyDefault,
        view::{ViewUniform, ViewUniforms},
    },
    utils::HashMap,
};

use crate::{
    material::OitMaterial,
    utils::{
        bind_group_layout_types::{storage_buffer, uniform_buffer},
        BindingResouceExt, RenderDeviceExt, RenderPipelineDescriptorBuilder,
    },
    OitLayersBindGroup, OIT_DRAW_SHADER_HANDLE, OIT_LAYERS, OIT_RENDER_SHADER_HANDLE,
};

#[derive(Resource)]
pub struct OitDrawPipeline {
    pub(crate) mesh_pipeline: MeshPipeline,
    pub(crate) oit_material_bind_group_layout: BindGroupLayout,
    pub(crate) oit_layers_bind_group_layout: BindGroupLayout,
}

impl FromWorld for OitDrawPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let oit_material_bind_group_layout = OitMaterial::bind_group_layout(render_device);

        let oit_layers_bind_group_layout = render_device.create_bind_group_layout_ext(
            "oit_layers_bind_group_layout",
            ShaderStages::FRAGMENT,
            [
                storage_buffer(false, false, None),
                storage_buffer(false, false, None),
            ],
        );

        let mesh_pipeline = world.resource::<MeshPipeline>().clone();

        OitDrawPipeline {
            mesh_pipeline,
            oit_material_bind_group_layout,
            oit_layers_bind_group_layout,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OitKey {
    pub mesh_key: MeshPipelineKey,
    pub tail_blend: bool,
}

impl SpecializedMeshPipeline for OitDrawPipeline {
    type Key = OitKey;
    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut desc = self.mesh_pipeline.specialize(key.mesh_key, layout)?;

        desc.label = Some("oit_draw_mesh_pipeline".into());

        let mut layout = match key.mesh_key.msaa_samples() {
            1 => vec![self.mesh_pipeline.view_layout.clone()],
            _ => vec![self.mesh_pipeline.view_layout_multisampled.clone()],
        };
        layout.push(self.oit_material_bind_group_layout.clone());
        layout.push(self.mesh_pipeline.mesh_layouts.model_only.clone());
        layout.push(self.oit_layers_bind_group_layout.clone());

        let mut defs = vec![
            ShaderDefVal::Int("OIT_LAYERS".to_string(), OIT_LAYERS as i32),
            ShaderDefVal::UInt("MSAA".to_string(), key.mesh_key.msaa_samples()),
        ];
        if key.tail_blend {
            defs.push(ShaderDefVal::from("TAIL_BLEND".to_string()));
        }

        desc.layout = layout;
        desc.vertex.shader = OIT_DRAW_SHADER_HANDLE.typed();
        desc.vertex.shader_defs.extend_from_slice(&defs);
        if let Some(frag) = desc.fragment.as_mut() {
            frag.shader = OIT_DRAW_SHADER_HANDLE.typed();
            frag.shader_defs.extend_from_slice(&defs);
            if let Some(target) = frag.targets[0].as_mut() {
                target.blend = Some(BlendState::ALPHA_BLENDING);
            }
        }
        desc.depth_stencil = Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: false,
            depth_compare: CompareFunction::GreaterEqual,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        });
        desc.multisample = MultisampleState {
            count: key.mesh_key.msaa_samples(),
            mask: !0,
            // TODO investigate how to use this for OIT
            alpha_to_coverage_enabled: false,
        };

        Ok(desc)
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
#[allow(clippy::type_complexity)]
pub struct OitBuffers(
    pub HashMap<Entity, (usize, StorageBuffer<Vec<UVec2>>, StorageBuffer<Vec<i32>>)>,
);

#[derive(Resource, Deref)]
pub struct OitRenderViewBindGroup(pub BindGroup);

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub fn queue_bind_groups(
    mut commands: Commands,
    pipeline: Res<OitDrawPipeline>,
    render_pipeline: Res<OitRenderPipeline>,
    render_device: Res<RenderDevice>,
    buffers: Res<OitBuffers>,
    view_uniforms: Res<ViewUniforms>,
) {
    for (entity, (_, layers, layer_ids)) in &buffers.0 {
        let bg = render_device.create_bind_group_ext(
            "oit_layers_bind_group",
            &pipeline.oit_layers_bind_group_layout,
            [layers.bind(), layer_ids.bind()],
        );
        commands.entity(*entity).insert(OitLayersBindGroup(bg));
    }

    let bind_group = render_device.create_bind_group_ext(
        "oit_render_params_bind_group",
        &render_pipeline.view_bind_group_layout,
        [view_uniforms.uniforms.bind()],
    );
    commands.insert_resource(OitRenderViewBindGroup(bind_group));
}

#[derive(Resource)]
pub struct OitRenderPipeline {
    view_bind_group_layout: BindGroupLayout,
}

impl FromWorld for OitRenderPipeline {
    fn from_world(world: &mut World) -> Self {
        let view_bind_group_layout = world
            .resource::<RenderDevice>()
            .create_bind_group_layout_ext(
                "oit_render_view_layout",
                ShaderStages::FRAGMENT,
                [uniform_buffer(true, Some(ViewUniform::min_size()))],
            );
        OitRenderPipeline {
            view_bind_group_layout,
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
    msaa: Res<Msaa>,
) {
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
            &[
                ShaderDefVal::Int("OIT_LAYERS".to_string(), OIT_LAYERS as i32),
                ShaderDefVal::Bool("MULTISAMPLED".to_string(), msaa.samples() > 1),
            ],
        )
        .multisample_state(MultisampleState {
            count: msaa.samples(),
            mask: !0,
            alpha_to_coverage_enabled: false,
        })
        .layout(vec![
            render_pipeline.view_bind_group_layout.clone(),
            draw_pipeline.oit_layers_bind_group_layout.clone(),
        ])
        .build();

    let pipeline_id = pipeline_cache.queue_render_pipeline(desc);
    commands.insert_resource(OitRenderPipelineId(pipeline_id));
}
