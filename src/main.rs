use bevy::{
    core_pipeline::core_3d,
    prelude::{shape::UVSphere, *},
    reflect::{TypePath, TypeUuid},
    render::{
        extract_component::{ExtractComponentPlugin, UniformComponentPlugin},
        render_graph::{RenderGraphApp, ViewNodeRunner},
        render_resource::{AsBindGroup, ShaderRef},
        RenderApp,
    },
    window::WindowResolution,
};
use camera_controller::{CameraController, CameraControllerPlugin};
use oit_node::OitNode;
use oit_phase::{OitMaterial, OitMesh, OitMeshPlugin, OitSettings};
use post_process_pass::{PostProcessNode, PostProcessPipeline, PostProcessSettings};

use crate::clear_pass::{ClearNode, ClearPipeline, ClearSettings};

mod camera_controller;
mod clear_pass;
mod oit_node;
mod oit_phase;
mod post_process_pass;
mod utils;

pub const WINDOW_WIDTH: usize = 1280;
pub const WINDOW_HEIGHT: usize = 720;
pub const OIT_LAYERS: usize = 8;

// TODO handle resize

fn main() {
    App::new()
        .insert_resource(Msaa::Off)
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32),
                    ..default()
                }),
                ..default()
            }),
            MaterialPlugin::<GoochMaterial>::default(),
            OitMeshPlugin,
            OitPlugin,
            CameraControllerPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, mat)
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
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 1.0, 5.0),
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

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    let pos_a = Vec3::new(-0.5, 0.5, 0.0);
    let pos_b = Vec3::new(0.0, 0.0, 0.0);
    let pos_c = Vec3::new(0.5, 0.5, 0.0);

    let offset = Vec3::new(1.65, 0.0, 0.0);

    // OIT
    commands.spawn(OitBundle {
        mesh: meshes.add(UVSphere::default().into()),
        material: OitMaterial {
            base_color: Color::RED.with_a(0.5),
        },
        transform: Transform::from_translation(pos_a - offset),
        ..default()
    });
    commands.spawn(OitBundle {
        mesh: meshes.add(UVSphere::default().into()),
        material: OitMaterial {
            base_color: Color::GREEN.with_a(0.5),
        },
        transform: Transform::from_translation(pos_b - offset),
        ..default()
    });
    commands.spawn(OitBundle {
        mesh: meshes.add(UVSphere::default().into()),
        material: OitMaterial {
            base_color: Color::BLUE.with_a(0.5),
        },
        transform: Transform::from_translation(pos_c - offset),
        ..default()
    });

    // Alpha Blend
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

    // Bunny
    // commands.spawn(SceneBundle {
    //     scene: asset_server.load("bunny.glb#Scene0"),
    //     ..default()
    // });
}

fn mat(mut commands: Commands, q: Query<Entity, With<Handle<StandardMaterial>>>) {
    for e in &q {
        commands
            .entity(e)
            .remove::<Handle<StandardMaterial>>()
            .insert((
                OitMaterial {
                    base_color: Color::BLUE.with_a(0.5),
                },
                OitMesh,
            ));
    }
}

pub const CLEAR_PASS: &str = "clear_pass";
pub const OIT_PASS: &str = "oit_pass";
pub const POST_PROCESS_PASS: &str = "post_process_pass";

struct OitPlugin;
impl Plugin for OitPlugin {
    fn build(&self, app: &mut App) {
        app
            // clear pass
            .add_plugins((
                ExtractComponentPlugin::<ClearSettings>::default(),
                UniformComponentPlugin::<ClearSettings>::default(),
            ))
            .add_systems(Update, clear_pass::update_settings)
            // oit phase
            .add_plugins((
                ExtractComponentPlugin::<OitMaterial>::default(),
                UniformComponentPlugin::<OitMaterial>::default(),
                ExtractComponentPlugin::<OitSettings>::default(),
                UniformComponentPlugin::<OitSettings>::default(),
            ))
            // post process
            .add_plugins((
                ExtractComponentPlugin::<PostProcessSettings>::default(),
                UniformComponentPlugin::<PostProcessSettings>::default(),
            ))
            .add_systems(Update, post_process_pass::update_settings);

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        const CORE_3D: &str = core_3d::graph::NAME;

        use core_3d::graph::node::*;
        render_app
            .add_render_graph_node::<ViewNodeRunner<ClearNode>>(CORE_3D, CLEAR_PASS)
            .add_render_graph_node::<ViewNodeRunner<OitNode>>(CORE_3D, OIT_PASS)
            .add_render_graph_node::<ViewNodeRunner<PostProcessNode>>(CORE_3D, POST_PROCESS_PASS)
            .add_render_graph_edges(CORE_3D, &[END_MAIN_PASS, OIT_PASS, POST_PROCESS_PASS])
            .add_render_graph_edges(CORE_3D, &[CLEAR_PASS, OIT_PASS]);
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<PostProcessPipeline>()
            .init_resource::<ClearPipeline>();
    }
}

#[derive(AsBindGroup, TypeUuid, TypePath, Debug, Clone)]
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
