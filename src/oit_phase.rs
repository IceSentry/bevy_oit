use bevy::{
    asset::load_internal_asset,
    ecs::{
        query::ROQueryItem,
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        },
    },
    pbr::{
        DrawMesh, MeshPipeline, MeshPipelineKey, MeshUniform, SetMeshBindGroup,
        SetMeshViewBindGroup,
    },
    prelude::*,
    reflect::TypeUuid,
    render::{
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
        },
        mesh::InnerMeshVertexBufferLayout,
        render_asset::RenderAssets,
        render_phase::{
            sort_phase_system, AddRenderCommand, CachedRenderPipelinePhaseItem, DrawFunctionId,
            DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupLayout, BindGroupLayoutDescriptor,
            BindingType, BufferBindingType, CachedRenderPipelineId, PipelineCache,
            RenderPipelineDescriptor, ShaderStages, ShaderType, SpecializedMeshPipeline,
            SpecializedMeshPipelineError, SpecializedMeshPipelines, StorageBuffer,
        },
        renderer::{RenderDevice, RenderQueue},
        view::{ExtractedView, VisibleEntities},
        Extract, Render, RenderApp, RenderSet,
    },
    utils::{FixedState, FloatOrd, Hashed},
};

use crate::{
    bind_group_entries, bind_group_layout_entries,
    utils::{color_target, fragment_state},
    OIT_LAYERS, WINDOW_HEIGHT, WINDOW_WIDTH,
};

pub const OIT_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 8527429857443840);

// TODO just use OitMaterial to identify the mesh
#[derive(Component, Clone, Copy, ExtractComponent, Default)]
pub struct OitMesh;

pub struct OitMeshPlugin;
impl Plugin for OitMeshPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, OIT_SHADER_HANDLE, "oit.wgsl", Shader::from_wgsl);
        app.add_plugins(ExtractComponentPlugin::<OitMesh>::default());

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<DrawFunctions<Oit>>()
            .add_render_command::<Oit, DrawOitMesh>()
            .add_systems(ExtractSchedule, extract_phase)
            .add_systems(
                Render,
                (
                    sort_phase_system::<Oit>.in_set(RenderSet::PhaseSort),
                    queue_bind_group.in_set(RenderSet::Queue),
                    queue_oit_mesh.in_set(RenderSet::Queue),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<OitPipeline>()
            .init_resource::<SpecializedMeshPipelines<OitPipeline>>();
    }
}

pub struct Oit {
    pub distance: f32,
    pub pipeline: CachedRenderPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for Oit {
    type SortKey = FloatOrd;

    fn entity(&self) -> Entity {
        self.entity
    }

    fn sort_key(&self) -> Self::SortKey {
        FloatOrd(self.distance)
    }

    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }
}

impl CachedRenderPipelinePhaseItem for Oit {
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub struct SetOitBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetOitBindGroup<I> {
    type Param = SRes<OitBindGroup>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<DynamicUniformIndex<OitMaterial>>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        entity: ROQueryItem<'w, Self::ItemWorldQuery>,
        oit_bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &oit_bind_group.into_inner().value, &[entity.index()]);
        RenderCommandResult::Success
    }
}

pub struct SetOitLayersBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetOitLayersBindGroup<I> {
    type Param = SRes<OitBindGroup>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<DynamicUniformIndex<OitMaterial>>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        entity: ROQueryItem<'w, Self::ItemWorldQuery>,
        oit_bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &oit_bind_group.into_inner().layers, &[entity.index()]);
        RenderCommandResult::Success
    }
}

pub type DrawOitMesh = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    SetOitBindGroup<2>,
    SetOitLayersBindGroup<3>,
    DrawMesh,
);

#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType)]
pub struct OitMaterial {
    pub base_color: Color,
}

#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType)]
pub struct OitSettings {
    pub oit_layers: u32,
}

#[derive(Resource)]
pub struct OitPipeline {
    mesh_pipeline: MeshPipeline,
    layout: BindGroupLayout,
    layers_layout: BindGroupLayout,
    pub counter_buffer: StorageBuffer<Vec<i32>>,
    pub layers: StorageBuffer<Vec<Vec4>>,
}

impl FromWorld for OitPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();

        let layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("oit_bind_group_layout"),
            entries: &bind_group_layout_entries![
                // material
                0 => (ShaderStages::VERTEX_FRAGMENT, BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(OitMaterial::min_size()),
                }),
                // settings
                1 => (ShaderStages::VERTEX_FRAGMENT, BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(OitSettings::min_size()),
                }),
            ],
        });

        let layers_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("oit_bind_group_layers_layout"),
            entries: &bind_group_layout_entries![
                // counter
                0 => (ShaderStages::VERTEX_FRAGMENT, BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                }),
                // layers
                1 => (ShaderStages::VERTEX_FRAGMENT, BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                }),
            ],
        });

        let mut counter_buffer = StorageBuffer::default();
        counter_buffer.set(vec![0; WINDOW_WIDTH * WINDOW_HEIGHT]);
        counter_buffer.write_buffer(render_device, render_queue);

        let mut layers = StorageBuffer::default();
        layers.set(vec![Vec4::ZERO; WINDOW_WIDTH * WINDOW_HEIGHT * OIT_LAYERS]);
        layers.write_buffer(render_device, render_queue);

        let mesh_pipeline = world.resource::<MeshPipeline>().clone();
        OitPipeline {
            mesh_pipeline,
            layout,
            layers_layout,
            counter_buffer,
            layers,
        }
    }
}

impl SpecializedMeshPipeline for OitPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &Hashed<InnerMeshVertexBufferLayout, FixedState>,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut desc = self.mesh_pipeline.specialize(key, layout)?;

        desc.label = Some("oit_mesh_pipeline".into());

        let mut layout = match key.msaa_samples() {
            1 => vec![self.mesh_pipeline.view_layout.clone()],
            _ => {
                vec![self.mesh_pipeline.view_layout_multisampled.clone()]
            }
        };

        layout.push(self.mesh_pipeline.mesh_layouts.model_only.clone());
        layout.push(self.layout.clone());
        layout.push(self.layers_layout.clone());

        desc.layout = layout;
        desc.vertex.shader = OIT_SHADER_HANDLE.typed::<Shader>();
        desc.fragment = fragment_state(
            OIT_SHADER_HANDLE.typed(),
            "fragment",
            &[color_target(None)],
            &[],
        );

        Ok(desc)
    }
}

pub fn extract_phase(mut commands: Commands, cameras: Extract<Query<Entity, With<Camera3d>>>) {
    for entity in cameras.iter() {
        commands
            .get_or_spawn(entity)
            .insert(RenderPhase::<Oit>::default());
    }
}

#[derive(Resource)]
pub struct OitBindGroup {
    value: BindGroup,
    layers: BindGroup,
}

pub fn queue_bind_group(
    mut commands: Commands,
    pipeline: Res<OitPipeline>,
    render_device: Res<RenderDevice>,
    material_uniforms: Res<ComponentUniforms<OitMaterial>>,
    settings_uniforms: Res<ComponentUniforms<OitSettings>>,
) {
    let Some(material_binding) = material_uniforms.uniforms().binding() else {
        return;
    };

    let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
        return;
    };

    let oit_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
        label: Some("oit_bind_group"),
        layout: &pipeline.layout,
        entries: &bind_group_entries![
            0 => material_binding.clone(),
            1 => settings_binding.clone(),
        ],
    });

    let oit_layers_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
        label: Some("oit_layers_bind_group"),
        layout: &pipeline.layers_layout,
        entries: &bind_group_entries![
            0 => pipeline.counter_buffer.binding().unwrap(),
            1 => pipeline.layers.binding().unwrap(),
        ],
    });

    commands.insert_resource(OitBindGroup {
        value: oit_bind_group,
        layers: oit_layers_bind_group,
    });
}

#[allow(clippy::too_many_arguments)]
pub fn queue_oit_mesh(
    draw_functions: Res<DrawFunctions<Oit>>,
    pipeline: Res<OitPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<OitPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    render_meshes: Res<RenderAssets<Mesh>>,
    oit_meshes: Query<(Entity, &Handle<Mesh>, &MeshUniform), With<OitMesh>>,
    mut views: Query<(&ExtractedView, &mut VisibleEntities, &mut RenderPhase<Oit>)>,
    msaa: Res<Msaa>,
) {
    let draw_function = draw_functions.read().get_id::<DrawOitMesh>().unwrap();

    for (view, visible_entities, mut phase) in views.iter_mut() {
        let view_matrix = view.transform.compute_matrix();
        let inv_view_row_2 = view_matrix.inverse().row(2);

        let view_key = MeshPipelineKey::from_msaa_samples(msaa.samples());

        for visible_entity in visible_entities.entities.iter().copied() {
            let Ok((entity, mesh_handle, mesh_uniform)) = oit_meshes.get(visible_entity) else {
                continue;
            };
            let Some(mesh) = render_meshes.get(mesh_handle) else {
                continue;
            };

            let key = MeshPipelineKey::from_primitive_topology(mesh.primitive_topology) | view_key;

            let Ok(pipeline) = pipelines.specialize(&pipeline_cache, &pipeline, key, &mesh.layout) else {
                continue;
            };

            phase.add(Oit {
                entity,
                pipeline,
                draw_function,
                distance: inv_view_row_2.dot(mesh_uniform.transform.col(3)),
            });
        }
    }
}
