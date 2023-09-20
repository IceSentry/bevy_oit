#![warn(clippy::pedantic)]
#![allow(clippy::type_complexity)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::cast_possible_wrap)]

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
        extract_component::{ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin},
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
    utils::FloatOrd,
};
use material::OitMaterial;
use pipeline::{OitBuffers, OitRenderPipeline};

use crate::{material::OitMaterialPlugin, node::OitNode, pipeline::OitDrawPipeline};

// TODO make this runtime configurable
pub const OIT_LAYERS: usize = 16;

pub mod material;
mod node;
mod pipeline;
mod utils;

#[allow(clippy::unreadable_literal)]
pub const OIT_DRAW_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 5948657369088000);

#[allow(clippy::unreadable_literal)]
pub const OIT_DRAW_BINDINGS_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 3431342664581120);

#[allow(clippy::unreadable_literal)]
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
            OIT_DRAW_BINDINGS_SHADER_HANDLE,
            "oit_draw_bindings.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            OIT_RENDER_SHADER_HANDLE,
            "oit_render.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins((
            UniformComponentPlugin::<OitMaterialUniform>::default(),
            ExtractComponentPlugin::<OitCamera>::default(),
            OitMaterialPlugin,
        ));

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
            .add_systems(ExtractSchedule, extract_render_phase)
            .add_systems(
                Render,
                (
                    sort_phase_system::<OitPhaseItem>.in_set(RenderSet::PhaseSort),
                    queue_mesh_oit_phase.in_set(RenderSet::Queue),
                    pipeline::queue_bind_groups.in_set(RenderSet::Queue),
                    pipeline::queue_render_oit_pipeline.in_set(RenderSet::Queue),
                ),
            );

        render_app
            .add_render_graph_node::<ViewNodeRunner<OitNode>>(CORE_3D, OitNode::NAME)
            .add_render_graph_edges(
                CORE_3D,
                &[core_3d::graph::node::MAIN_TRANSPARENT_PASS, OitNode::NAME],
            );
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<OitDrawPipeline>()
            .init_resource::<OitRenderPipeline>();
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

struct SetOitMaterialBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetOitMaterialBindGroup<I> {
    type Param = SRes<RenderAssets<OitMaterial>>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<Handle<OitMaterial>>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        material_handle: ROQueryItem<'w, Self::ItemWorldQuery>,
        materials: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(material) = materials.into_inner().get(material_handle) else {
            return RenderCommandResult::Failure;
        };
        pass.set_bind_group(I, &material.bind_group, &[]);
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
pub struct OitDepthBindGroup(pub BindGroup);

struct SetOitDepthBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetOitDepthBindGroup<I> {
    type Param = ();
    type ViewWorldQuery = &'static OitDepthBindGroup;
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
    SetOitDepthBindGroup<4>,
    DrawMesh,
);

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

#[allow(clippy::too_many_arguments)]
fn queue_mesh_oit_phase(
    draw_functions: Res<DrawFunctions<OitPhaseItem>>,
    pipeline: Res<OitDrawPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<OitDrawPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    render_meshes: Res<RenderAssets<Mesh>>,
    meshes: Query<(Entity, &Handle<Mesh>, &MeshUniform), With<Handle<OitMaterial>>>,
    mut views: Query<(
        &ExtractedView,
        &mut VisibleEntities,
        &mut RenderPhase<OitPhaseItem>,
    )>,
    msaa: Res<Msaa>,
) {
    let draw_function = draw_functions.read().id::<DrawOit>();

    for (view, visible_entities, mut oit_phase) in &mut views {
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
        let Some(size) = camera.physical_target_size else {
            continue;
        };

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
            // Maybe hide the OIT pass while resizing or keep it centered somehow?

            oit_layers_buffer
                .get_mut()
                .resize(size * OIT_LAYERS, UVec2::ZERO);
            oit_layers_buffer.write_buffer(&render_device, &render_queue);

            oit_layer_ids_buffer.get_mut().resize(size, 0);
            oit_layer_ids_buffer.write_buffer(&render_device, &render_queue);

            *current_size = size;
        } else {
            // init buffers
            let mut oit_layers_buffer = StorageBuffer::default();
            oit_layers_buffer.set(vec![UVec2::ZERO; size * OIT_LAYERS]);
            oit_layers_buffer.write_buffer(&render_device, &render_queue);

            let mut oit_layer_ids_buffer = StorageBuffer::default();
            oit_layer_ids_buffer.set(vec![0; size]);
            oit_layer_ids_buffer.write_buffer(&render_device, &render_queue);

            buffers.insert(entity, (size, oit_layers_buffer, oit_layer_ids_buffer));
        }
    }
}
