use bevy::{
    core_pipeline::core_3d,
    pbr::NotShadowCaster,
    prelude::{shape::UVSphere, *},
    reflect::TypeUuid,
    render::{
        extract_component::{ExtractComponentPlugin, UniformComponentPlugin},
        render_resource::{AsBindGroup, ShaderRef},
        RenderApp,
    },
    window::WindowResolution,
};
use camera_controller::{CameraController, CameraControllerPlugin};
use oit_node::OitNode;
use oit_phase::{OitMaterial, OitMesh, OitMeshPlugin, OitSettings};
use post_process_pass::{PostProcessNode, PostProcessPipeline, PostProcessSettings};
use utils::render_graph_app::*;

use crate::clear_pass::{ClearNode, ClearPipeline, ClearSettings};

mod camera_controller;
mod clear_pass;
mod oit_node;
mod oit_phase;
mod post_process_pass;
mod utils;

pub const WINDOW_WIDTH: usize = 1280;
pub const WINDOW_HEIGHT: usize = 720;
pub const OIT_LAYERS: usize = 16;

fn main() {
    App::new()
        .insert_resource(Msaa::Off)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32),
                ..default()
            }),
            ..default()
        }))
        .add_plugin(MaterialPlugin::<GoochMaterial>::default())
        .add_plugin(OitMeshPlugin)
        .add_plugin(OitPlugin)
        .add_plugin(CameraControllerPlugin)
        .add_startup_system(setup)
        .run();
}

#[derive(Bundle, Clone, Default)]
pub struct OitBundle {
    pub mesh: Handle<Mesh>,
    pub material: OitMaterial,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
    pub tag: OitMesh,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GoochMaterial>>,
) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 5.0),
            ..default()
        },
        PostProcessSettings {
            viewport_width: WINDOW_WIDTH as f32,
            viewport_height: WINDOW_HEIGHT as f32,
            oit_layers: OIT_LAYERS as u32,
        },
        OitSettings {
            oit_layers: OIT_LAYERS as u32,
        },
        ClearSettings {
            viewport_width: WINDOW_WIDTH as f32,
        },
        CameraController::default(),
    ));

    let pos_a = Vec3::new(-0.5, 0.5, -0.25);
    let pos_b = Vec3::new(0.0, 0.0, 0.0);
    let pos_c = Vec3::new(0.5, 0.5, 0.25);

    let offset = Vec3::new(-1.65, 0.0, 0.0);
    commands.spawn(OitBundle {
        mesh: meshes.add(UVSphere::default().into()),
        material: OitMaterial {
            base_color: Color::RED.with_a(0.5),
        },
        transform: Transform::from_translation(pos_a + offset),
        ..default()
    });
    commands.spawn(OitBundle {
        mesh: meshes.add(UVSphere::default().into()),
        material: OitMaterial {
            base_color: Color::GREEN.with_a(0.5),
        },
        transform: Transform::from_translation(pos_b + offset),
        ..default()
    });
    commands.spawn(OitBundle {
        mesh: meshes.add(UVSphere::default().into()),
        material: OitMaterial {
            base_color: Color::BLUE.with_a(0.5),
        },
        transform: Transform::from_translation(pos_c + offset),
        ..default()
    });

    let offset = Vec3::new(1.65, 0.0, 0.0);
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(UVSphere::default().into()),
        material: materials.add(GoochMaterial {
            base_color: Color::RED.with_a(0.5),
        }),
        transform: Transform::from_translation(pos_a + offset),
        ..default()
    });
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(UVSphere::default().into()),
        material: materials.add(GoochMaterial {
            base_color: Color::GREEN.with_a(0.5),
        }),
        transform: Transform::from_translation(pos_b + offset),
        ..default()
    });
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(UVSphere::default().into()),
        material: materials.add(GoochMaterial {
            base_color: Color::BLUE.with_a(0.5),
        }),
        transform: Transform::from_translation(pos_c + offset),
        ..default()
    });
}

pub const CLEAR_PASS: &str = "clear_pass";
pub const OIT_PASS: &str = "oit_pass";
pub const POST_PROCESS_PASS: &str = "post_process_pass";

struct OitPlugin;
impl Plugin for OitPlugin {
    fn build(&self, app: &mut App) {
        app
            // clear pass
            .add_plugin(ExtractComponentPlugin::<ClearSettings>::default())
            .add_plugin(UniformComponentPlugin::<ClearSettings>::default())
            .add_system(clear_pass::update_settings)
            // oit phase
            .add_plugin(ExtractComponentPlugin::<OitMaterial>::default())
            .add_plugin(UniformComponentPlugin::<OitMaterial>::default())
            .add_plugin(ExtractComponentPlugin::<OitSettings>::default())
            .add_plugin(UniformComponentPlugin::<OitSettings>::default())
            // post process
            .add_plugin(ExtractComponentPlugin::<PostProcessSettings>::default())
            .add_plugin(UniformComponentPlugin::<PostProcessSettings>::default())
            .add_system(post_process_pass::update_settings);

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<PostProcessPipeline>()
            .init_resource::<ClearPipeline>();

        const CORE_3D: &str = core_3d::graph::NAME;

        use core_3d::graph::node::*;
        render_app
            .add_view_node::<ClearNode>(CORE_3D, CLEAR_PASS)
            .add_view_node::<OitNode>(CORE_3D, OIT_PASS)
            .add_view_node::<PostProcessNode>(CORE_3D, POST_PROCESS_PASS)
            .add_render_graph_edges(CORE_3D, &[MAIN_PASS, OIT_PASS, POST_PROCESS_PASS])
            .add_render_graph_edges(CORE_3D, &[CLEAR_PASS, OIT_PASS]);
    }
}

#[derive(AsBindGroup, TypeUuid, Debug, Clone)]
#[uuid = "fd884c25-98b1-5155-a809-881b0740b498"]
struct GoochMaterial {
    #[uniform(0)]
    base_color: Color,
}

impl Material for GoochMaterial {
    fn fragment_shader() -> ShaderRef {
        "gooch_shading.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
