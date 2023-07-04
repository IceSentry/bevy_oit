use bevy::{
    core_pipeline::core_3d,
    prelude::{shape::UVSphere, *},
    render::{
        extract_component::{ExtractComponentPlugin, UniformComponentPlugin},
        render_graph::RenderGraph,
        RenderApp,
    },
    window::WindowResolution,
};
use oit_node::OitNode;
use oit_phase::{OitMaterial, OitMesh, OitMeshPlugin};
use post_process_pass::{
    update_settings, PostProcessNode, PostProcessPipeline, PostProcessSettings,
};
use utils::{render_graph_app::*, view_node::ViewNodeRunner};

mod oit_node;
mod oit_phase;
mod post_process_pass;
mod utils;

pub const WINDOW_WIDTH: usize = 1280;
pub const WINDOW_HEIGHT: usize = 720;
pub const OIT_LAYERS: usize = 1;

fn main() {
    App::new()
        .insert_resource(Msaa::Sample4)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32),
                ..default()
            }),
            ..default()
        }))
        .add_plugin(OitMeshPlugin)
        .add_plugin(OitPlugin)
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

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 5.0),
            ..default()
        },
        PostProcessSettings {
            intensity: 0.02,
            viewport_width: WINDOW_WIDTH as f32,
        },
    ));

    commands.spawn(OitBundle {
        mesh: meshes.add(UVSphere::default().into()),
        material: OitMaterial {
            base_color: Color::RED.with_a(0.5),
        },
        transform: Transform::from_xyz(-0.5, 0.5, -0.25),
        ..default()
    });
    commands.spawn(OitBundle {
        mesh: meshes.add(UVSphere::default().into()),
        material: OitMaterial {
            base_color: Color::GREEN.with_a(0.5),
        },
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });
    commands.spawn(OitBundle {
        mesh: meshes.add(UVSphere::default().into()),
        material: OitMaterial {
            base_color: Color::BLUE.with_a(0.5),
        },
        transform: Transform::from_xyz(0.5, 0.5, 0.25),
        ..default()
    });
}

pub const OIT_PASS: &str = "oit_pass";
pub const POST_PROCESS_PASS: &str = "post_process_pass";

struct OitPlugin;
impl Plugin for OitPlugin {
    fn build(&self, app: &mut App) {
        app
            // post process
            .add_plugin(ExtractComponentPlugin::<PostProcessSettings>::default())
            .add_plugin(UniformComponentPlugin::<PostProcessSettings>::default())
            .add_system(update_settings)
            // oit phase
            .add_plugin(ExtractComponentPlugin::<OitMaterial>::default())
            .add_plugin(UniformComponentPlugin::<OitMaterial>::default());

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<PostProcessPipeline>();

        const CORE_3D: &str = core_3d::graph::NAME;

        use core_3d::graph::node::*;
        render_app
            .add_view_node::<PostProcessNode>(CORE_3D, POST_PROCESS_PASS)
            .add_view_node::<OitNode>(CORE_3D, OIT_PASS)
            .add_render_graph_edges(CORE_3D, &[MAIN_PASS, OIT_PASS, POST_PROCESS_PASS]);
    }
}
