use bevy::{
    pbr::{MeshPipeline, MeshPipelineKey},
    prelude::*,
    render::{
        extract_component::ComponentUniforms,
        mesh::MeshVertexBufferLayout,
        render_resource::{
            BindGroup, BindGroupLayout, BlendComponent, BlendState, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, MultisampleState, PipelineCache,
            RenderPipelineDescriptor, ShaderDefVal, ShaderStages, ShaderType,
            SpecializedMeshPipeline, SpecializedMeshPipelineError, StorageBuffer, TextureFormat,
        },
        renderer::RenderDevice,
        texture::BevyDefault,
        view::{ViewDepthTexture, ViewUniform, ViewUniforms},
    },
    utils::HashMap,
};

use crate::{
    utils::{
        bind_group_layout_types::{storage_buffer, texture_depth_2d, uniform_buffer},
        BindingResouceExt, RenderDeviceExt, RenderPipelineDescriptorBuilder,
    },
    OitCamera, OitDepthBindGroup, OitLayersBindGroup, OitMaterialUniform,
    OitMaterialUniformsBindGroup, OIT_DRAW_SHADER_HANDLE, OIT_LAYERS, OIT_RENDER_SHADER_HANDLE,
};

#[derive(Resource)]
pub struct OitDrawPipeline {
    pub(crate) mesh_pipeline: MeshPipeline,
    pub(crate) oit_material_bind_group_layout: BindGroupLayout,
    pub(crate) oit_layers_bind_group_layout: BindGroupLayout,
    pub(crate) depth_bind_group_layout: BindGroupLayout,
    pub(crate) depth_bind_group_layout_mutlisampled: BindGroupLayout,
}

impl FromWorld for OitDrawPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let oit_material_bind_group_layout = render_device.create_bind_group_layout_ext(
            "oit_material_bind_group_layout",
            ShaderStages::FRAGMENT,
            [uniform_buffer(true, Some(OitMaterialUniform::min_size()))],
        );

        let oit_layers_bind_group_layout = render_device.create_bind_group_layout_ext(
            "oit_layers_bind_group_layout",
            ShaderStages::FRAGMENT,
            [
                storage_buffer(false, false, None),
                storage_buffer(false, false, None),
            ],
        );

        let depth_bind_group_layout = render_device.create_bind_group_layout_ext(
            "depth_bind_group_layout",
            ShaderStages::FRAGMENT,
            [texture_depth_2d(false)],
        );

        let depth_bind_group_layout_mutlisampled = render_device.create_bind_group_layout_ext(
            "depth_bind_group_layout_multisampled",
            ShaderStages::FRAGMENT,
            [texture_depth_2d(true)],
        );

        let mesh_pipeline = world.resource::<MeshPipeline>().clone();

        OitDrawPipeline {
            mesh_pipeline,
            oit_material_bind_group_layout,
            oit_layers_bind_group_layout,
            depth_bind_group_layout,
            depth_bind_group_layout_mutlisampled,
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
        match key.msaa_samples() {
            1 => layout.push(self.depth_bind_group_layout.clone()),
            _ => layout.push(self.depth_bind_group_layout_mutlisampled.clone()),
        };

        let defs = vec![
            ShaderDefVal::Int("OIT_LAYERS".to_string(), OIT_LAYERS as i32),
            ShaderDefVal::UInt("MSAA".to_string(), key.msaa_samples()),
        ];

        desc.layout = layout;
        desc.vertex.shader = OIT_DRAW_SHADER_HANDLE.typed();
        desc.vertex.shader_defs.extend_from_slice(&defs);
        if let Some(frag) = desc.fragment.as_mut() {
            frag.shader = OIT_DRAW_SHADER_HANDLE.typed();
            frag.shader_defs.extend_from_slice(&defs);
        }
        desc.depth_stencil = None;
        desc.multisample = MultisampleState {
            count: key.msaa_samples(),
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
    material_uniforms: Res<ComponentUniforms<OitMaterialUniform>>,
    buffers: Res<OitBuffers>,
    view_uniforms: Res<ViewUniforms>,
    depth_textures: Query<(Entity, &ViewDepthTexture), (With<Camera3d>, With<OitCamera>)>,
    msaa: Res<Msaa>,
) {
    let material_uniforms_bind_group = render_device.create_bind_group_ext(
        "oit_material_bind_group",
        &pipeline.oit_material_bind_group_layout,
        [material_uniforms.uniforms().bind()],
    );
    commands.insert_resource(OitMaterialUniformsBindGroup(material_uniforms_bind_group));

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

    for (e, texture) in &depth_textures {
        let layout = match msaa.samples() {
            1 => &pipeline.depth_bind_group_layout,
            _ => &pipeline.depth_bind_group_layout_mutlisampled,
        };
        let bind_group = render_device.create_bind_group_ext(
            "oit_draw_depth_bind_group",
            layout,
            [texture.view.bind()],
        );
        commands.entity(e).insert(OitDepthBindGroup(bind_group));
    }
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
