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
        extract_component::{DynamicUniformIndex, UniformComponentPlugin},
        render_asset::RenderAssets,
        render_graph::{RenderGraphApp, ViewNodeRunner},
        render_phase::{
            sort_phase_system, AddRenderCommand, CachedRenderPipelinePhaseItem, DrawFunctionId,
            DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            BindGroup, CachedRenderPipelineId, PipelineCache, ShaderType, SpecializedMeshPipelines,
        },
        view::{ExtractedView, VisibleEntities},
        Extract, Render, RenderApp, RenderSet,
    },
    utils::FloatOrd,
};

use crate::{node::OitNode, pipeline::OitDrawPipeline};

pub const WINDOW_WIDTH: usize = 1280;
pub const WINDOW_HEIGHT: usize = 720;
pub const OIT_LAYERS: usize = 8;

mod node;
mod pipeline;
mod utils;

pub const OIT_DRAW_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 5948657369088000);

pub const OIT_RENDER_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1612685519093760);

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

        app.add_plugins(UniformComponentPlugin::<OitMaterialUniform>::default());

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedMeshPipelines<OitDrawPipeline>>()
            .init_resource::<DrawFunctions<OitPhaseItem>>()
            .add_render_command::<OitPhaseItem, DrawOit>();

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

struct SetMaterialBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetMaterialBindGroup<I> {
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

#[derive(Resource, Deref)]
pub(crate) struct OitLayersBindGroup(pub(crate) BindGroup);

struct SetOitLayersBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetOitLayersBindGroup<I> {
    type Param = SRes<OitLayersBindGroup>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        _mesh_index: ROQueryItem<'w, Self::ItemWorldQuery>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, bind_group.into_inner(), &[]);
        RenderCommandResult::Success
    }
}

#[derive(Resource, Deref)]
pub(crate) struct OitLayerIdsBindGroup(pub(crate) BindGroup);

struct SetOitLayerIdsBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetOitLayerIdsBindGroup<I> {
    type Param = SRes<OitLayerIdsBindGroup>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        _mesh_index: ROQueryItem<'w, Self::ItemWorldQuery>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, bind_group.into_inner(), &[]);
        RenderCommandResult::Success
    }
}

type DrawOit = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMaterialBindGroup<1>,
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

            let Ok(pipeline) = pipelines.specialize(&pipeline_cache, &pipeline, key, &mesh.layout) else {
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
