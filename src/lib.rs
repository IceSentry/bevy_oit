use bevy::{
    asset::load_internal_asset,
    core_pipeline::core_3d::{self, CORE_3D},
    ecs::{
        query::ROQueryItem,
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        },
    },
    pbr::{DrawMesh, MeshPipelineKey, MeshUniform, SetMeshBindGroup, SetMeshViewBindGroup},
    prelude::*,
    reflect::TypeUuid,
    render::{
        camera::ExtractedCamera,
        extract_component::{
            DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin,
        },
        render_asset::RenderAssets,
        render_graph::{RenderGraphApp, ViewNodeRunner},
        render_phase::{
            sort_phase_system, AddRenderCommand, CachedRenderPipelinePhaseItem, DrawFunctionId,
            DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            BindGroup, CachedRenderPipelineId, PipelineCache, ShaderType, SpecializedMeshPipelines,
            StorageBuffer,
        },
        renderer::{RenderDevice, RenderQueue},
        view::{ExtractedView, VisibleEntities},
        Extract, Render, RenderApp, RenderSet,
    },
    utils::{FloatOrd, HashMap},
};
use pipeline::OitBuffers;

use crate::{node::OitNode, pipeline::OitDrawPipeline};

pub const WINDOW_WIDTH: usize = 1280;
pub const WINDOW_HEIGHT: usize = 720;
pub const OIT_LAYERS: usize = 16;

mod node;
mod pipeline;
mod utils;

pub const OIT_DRAW_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 5948657369088000);

pub const OIT_RENDER_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1612685519093760);

#[derive(Component, Clone, Copy, ExtractComponent)]
pub struct OitCamera;

pub struct OitPlugin;
impl Plugin for OitPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            OIT_DRAW_SHADER_HANDLE,
            "oit_draw.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            OIT_RENDER_SHADER_HANDLE,
            "oit_render.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins(UniformComponentPlugin::<OitMaterialUniform>::default())
            .add_plugins(ExtractComponentPlugin::<OitCamera>::default());

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedMeshPipelines<OitDrawPipeline>>()
            .init_resource::<DrawFunctions<OitPhaseItem>>()
            .init_resource::<OitBuffers>()
            .add_render_command::<OitPhaseItem, DrawOit>()
            .add_systems(Render, prepare_buffers.in_set(RenderSet::Prepare));

        render_app
            .add_systems(
                ExtractSchedule,
                (extract_render_phase, extract_oit_material_uniform),
            )
            .add_systems(
                Render,
                (
                    sort_phase_system::<OitPhaseItem>.in_set(RenderSet::PhaseSort),
                    queue_mesh_oit_phase.in_set(RenderSet::Queue),
                    pipeline::queue_bind_group.in_set(RenderSet::Queue),
                    pipeline::queue_render_oit_pipeline.in_set(RenderSet::Queue),
                ),
            );

        render_app
            .add_render_graph_node::<ViewNodeRunner<OitNode>>(CORE_3D, OitNode::NAME)
            .add_render_graph_edges(
                CORE_3D,
                &[
                    core_3d::graph::node::MAIN_OPAQUE_PASS,
                    OitNode::NAME,
                    core_3d::graph::node::MAIN_TRANSPARENT_PASS,
                ],
            );
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<OitDrawPipeline>();
    }
}

pub struct OitPhaseItem {
    pub distance: f32,
    pub pipeline: CachedRenderPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for OitPhaseItem {
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

impl CachedRenderPipelinePhaseItem for OitPhaseItem {
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

#[derive(Resource, Deref)]
pub(crate) struct OitMaterialUniformsBindGroup(pub(crate) BindGroup);

struct SetOitMaterialBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetOitMaterialBindGroup<I> {
    type Param = SRes<OitMaterialUniformsBindGroup>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<DynamicUniformIndex<OitMaterialUniform>>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        mesh_index: ROQueryItem<'w, Self::ItemWorldQuery>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, bind_group.into_inner(), &[mesh_index.index()]);
        RenderCommandResult::Success
    }
}

#[derive(Component, Deref)]
pub struct OitLayersBindGroup(pub BindGroup);

struct SetOitLayersBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetOitLayersBindGroup<I> {
    type Param = ();
    type ViewWorldQuery = &'static OitLayersBindGroup;
    type ItemWorldQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        bind_group: ROQueryItem<'w, Self::ViewWorldQuery>,
        _mesh_index: ROQueryItem<'w, Self::ItemWorldQuery>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, bind_group, &[]);
        RenderCommandResult::Success
    }
}

#[derive(Component, Deref)]
pub struct OitLayerIdsBindGroup(pub BindGroup);

struct SetOitLayerIdsBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetOitLayerIdsBindGroup<I> {
    type Param = ();
    type ViewWorldQuery = &'static OitLayerIdsBindGroup;
    type ItemWorldQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        bind_group: ROQueryItem<'w, Self::ViewWorldQuery>,
        _mesh_index: ROQueryItem<'w, Self::ItemWorldQuery>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, bind_group, &[]);
        RenderCommandResult::Success
    }
}

type DrawOit = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetOitMaterialBindGroup<1>,
    SetMeshBindGroup<2>,
    SetOitLayersBindGroup<3>,
    SetOitLayerIdsBindGroup<4>,
    DrawMesh,
);

#[derive(Component, Clone, Copy, Default)]
pub struct OitMaterial {
    pub base_color: Color,
}

#[derive(Bundle, Clone, Default)]
pub struct OitMaterialMeshBundle {
    pub mesh: Handle<Mesh>,
    pub material: OitMaterial,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
}

#[derive(Component, ShaderType, Clone, Copy)]
pub struct OitMaterialUniform {
    base_color: Color,
}

fn extract_render_phase(
    mut commands: Commands,
    cameras_3d: Extract<Query<(Entity, &Camera), With<Camera3d>>>,
) {
    for (entity, camera) in &cameras_3d {
        if camera.is_active {
            commands
                .get_or_spawn(entity)
                .insert(RenderPhase::<OitPhaseItem>::default());
        }
    }
}

fn extract_oit_material_uniform(
    mut commands: Commands,
    oit_materials: Extract<Query<(Entity, &OitMaterial)>>,
) {
    for (entity, material) in &oit_materials {
        commands.get_or_spawn(entity).insert(OitMaterialUniform {
            base_color: material.base_color,
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn queue_mesh_oit_phase(
    draw_functions: Res<DrawFunctions<OitPhaseItem>>,
    pipeline: Res<OitDrawPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<OitDrawPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    render_meshes: Res<RenderAssets<Mesh>>,
    meshes: Query<(Entity, &Handle<Mesh>, &MeshUniform), With<OitMaterialUniform>>,
    mut views: Query<(
        &ExtractedView,
        &mut VisibleEntities,
        &mut RenderPhase<OitPhaseItem>,
    )>,
    msaa: Res<Msaa>,
) {
    let draw_function = draw_functions.read().id::<DrawOit>();

    for (view, visible_entities, mut oit_phase) in views.iter_mut() {
        let view_matrix = view.transform.compute_matrix();
        let inv_view_row_2 = view_matrix.inverse().row(2);

        let view_key = MeshPipelineKey::from_msaa_samples(msaa.samples());

        for visible_entity in visible_entities.entities.iter().copied() {
            let Ok((entity, mesh_handle, mesh_uniform)) = meshes.get(visible_entity) else {
                continue;
            };
            let Some(mesh) = render_meshes.get(mesh_handle) else {
                continue;
            };

            let key = MeshPipelineKey::from_primitive_topology(mesh.primitive_topology) | view_key;

            let Ok(pipeline) = pipelines.specialize(&pipeline_cache, &pipeline, key, &mesh.layout)
            else {
                continue;
            };

            oit_phase.add(OitPhaseItem {
                entity,
                pipeline,
                draw_function,
                distance: inv_view_row_2.dot(mesh_uniform.transform.col(3)),
            });
        }
    }
}

/// This creates the required buffers for each camera
#[allow(clippy::type_complexity)]
fn prepare_buffers(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    cameras: Query<(Entity, &ExtractedCamera), (Changed<ExtractedCamera>, With<OitCamera>)>,
    mut buffers: ResMut<OitBuffers>,
) {
    for (entity, camera) in &cameras {
        let Some(size) = camera.physical_target_size else { continue; };

        let size = (size.x * size.y) as usize;

        if let Some((current_size, oit_layers_buffer, oit_layer_ids_buffer)) =
            buffers.get_mut(&entity)
        {
            // resize buffers
            if *current_size >= size {
                // Don't resize if the buffer is already bigger
                // This is technically wasting memory but it's a bit faster so...
                return;
            }

            println!("curr: {current_size} new: {size}");

            // TODO this is super slow, figure out a more efficient way to resize
            // Consider debouncing

            oit_layers_buffer
                .get_mut()
                .resize(size * OIT_LAYERS, Vec4::ZERO);
            oit_layers_buffer.write_buffer(&render_device, &render_queue);

            oit_layer_ids_buffer.get_mut().resize(size, 0);
            oit_layer_ids_buffer.write_buffer(&render_device, &render_queue);

            *current_size = size;
        } else {
            // init buffers
            let mut oit_layers_buffer = StorageBuffer::default();
            oit_layers_buffer.set(vec![Vec4::ZERO; size * OIT_LAYERS]);
            oit_layers_buffer.write_buffer(&render_device, &render_queue);

            let mut oit_layer_ids_buffer = StorageBuffer::default();
            oit_layer_ids_buffer.set(vec![0; size]);
            oit_layer_ids_buffer.write_buffer(&render_device, &render_queue);

            buffers.insert(entity, (size, oit_layers_buffer, oit_layer_ids_buffer));
        }
    }
}
