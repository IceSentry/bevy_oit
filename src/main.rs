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

pub mod graph {
    pub mod input {
        pub const VIEW_ENTITY: &str = "view_entity";
    }

    pub mod node {
        pub const OIT_PASS: &str = "oit_pass";
        pub const POST_PROCESS_PASS: &str = "post_process_pass";
    }
}

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

        // oit pass
        {
            let oit_node = OitNode::new(&mut render_app.world);
            let mut graph = render_app.world.resource_mut::<RenderGraph>();

            let core_3d_graph = graph.get_sub_graph_mut(core_3d::graph::NAME).unwrap();
            core_3d_graph.add_node(graph::node::OIT_PASS, oit_node);
            core_3d_graph.add_slot_edge(
                core_3d_graph.input_node().id,
                graph::input::VIEW_ENTITY,
                graph::node::OIT_PASS,
                OitNode::IN_VIEW,
            );
        }

        //post process
        {
            let node = PostProcessNode::new(&mut render_app.world);
            let mut graph = render_app.world.resource_mut::<RenderGraph>();
            let core_3d_graph = graph.get_sub_graph_mut(core_3d::graph::NAME).unwrap();
            core_3d_graph.add_node(graph::node::POST_PROCESS_PASS, node);
            core_3d_graph.add_slot_edge(
                core_3d_graph.input_node().id,
                core_3d::graph::input::VIEW_ENTITY,
                graph::node::POST_PROCESS_PASS,
                PostProcessNode::IN_VIEW,
            );
        }

        {
            let mut graph = render_app.world.resource_mut::<RenderGraph>();
            let core_3d_graph = graph.get_sub_graph_mut(core_3d::graph::NAME).unwrap();
            // MAIN -> OIT -> POST_PROCESS
            core_3d_graph.add_node_edge(core_3d::graph::node::MAIN_PASS, graph::node::OIT_PASS);
            core_3d_graph.add_node_edge(graph::node::OIT_PASS, graph::node::POST_PROCESS_PASS);
        }
    }
}
