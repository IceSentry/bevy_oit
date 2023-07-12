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

use crate::custom_phase::node::CustomNode;

use self::pipeline::CustomPipeline;

mod node;
mod pipeline;

const CUSTOM_DRAW_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 5948657369088000);

pub struct CustomRenderPhasePlugin;
impl Plugin for CustomRenderPhasePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            CUSTOM_DRAW_SHADER_HANDLE,
            "custom_draw.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins(UniformComponentPlugin::<CustomMaterialUniform>::default());

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedMeshPipelines<CustomPipeline>>()
            .init_resource::<DrawFunctions<CustomPhaseItem>>()
            .add_render_command::<CustomPhaseItem, DrawCustom>();

        render_app
            .add_systems(
                ExtractSchedule,
                (extract_render_phase, extract_custom_material_uniform),
            )
            .add_systems(
                Render,
                (
                    sort_phase_system::<CustomPhaseItem>.in_set(RenderSet::PhaseSort),
                    queue_mesh_custom_phase.in_set(RenderSet::Queue),
                    pipeline::queue_bind_group.in_set(RenderSet::Queue),
                ),
            );

        render_app
            .add_render_graph_node::<ViewNodeRunner<CustomNode>>(CORE_3D, CustomNode::NAME)
            .add_render_graph_edges(
                CORE_3D,
                &[
                    core_3d::graph::node::MAIN_OPAQUE_PASS,
                    CustomNode::NAME,
                    core_3d::graph::node::MAIN_TRANSPARENT_PASS,
                ],
            );
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<CustomPipeline>();
    }
}

pub struct CustomPhaseItem {
    pub distance: f32,
    pub pipeline: CachedRenderPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for CustomPhaseItem {
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

impl CachedRenderPipelinePhaseItem for CustomPhaseItem {
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

#[derive(Resource, Deref)]
struct CustomMaterialBindGroup(BindGroup);

struct SetMaterialBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetMaterialBindGroup<I> {
    type Param = SRes<CustomMaterialBindGroup>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<DynamicUniformIndex<CustomMaterialUniform>>;

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
struct OitLayersBindGroup(BindGroup);
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
struct OitLayerIdsBindGroup(BindGroup);
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

type DrawCustom = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMaterialBindGroup<1>,
    SetMeshBindGroup<2>,
    SetOitLayersBindGroup<3>,
    SetOitLayerIdsBindGroup<4>,
    DrawMesh,
);

#[derive(Component, Clone, Copy, Default)]
pub struct CustomMaterial {
    pub base_color: Color,
}

#[derive(Bundle, Clone, Default)]
pub struct CustomMaterialMeshBundle {
    pub mesh: Handle<Mesh>,
    pub material: CustomMaterial,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
}

#[derive(Component, ShaderType, Clone, Copy)]
pub struct CustomMaterialUniform {
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
                .insert(RenderPhase::<CustomPhaseItem>::default());
        }
    }
}

fn extract_custom_material_uniform(
    mut commands: Commands,
    custom_materials: Extract<Query<(Entity, &CustomMaterial)>>,
) {
    for (entity, custom_material) in &custom_materials {
        commands.get_or_spawn(entity).insert(CustomMaterialUniform {
            base_color: custom_material.base_color,
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn queue_mesh_custom_phase(
    draw_functions: Res<DrawFunctions<CustomPhaseItem>>,
    pipeline: Res<CustomPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<CustomPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    render_meshes: Res<RenderAssets<Mesh>>,
    meshes: Query<(Entity, &Handle<Mesh>, &MeshUniform), With<CustomMaterialUniform>>,
    mut views: Query<(
        &ExtractedView,
        &mut VisibleEntities,
        &mut RenderPhase<CustomPhaseItem>,
    )>,
    msaa: Res<Msaa>,
) {
    let draw_function = draw_functions.read().id::<DrawCustom>();

    for (view, visible_entities, mut custom_phase) in views.iter_mut() {
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

            custom_phase.add(CustomPhaseItem {
                entity,
                pipeline,
                draw_function,
                distance: inv_view_row_2.dot(mesh_uniform.transform.col(3)),
            });
        }
    }
}
