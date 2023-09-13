use bevy::{
    prelude::{shape::UVSphere, *},
    render::render_resource::TextureUsages,
    window::PresentMode,
};
use bevy_oit::{OitCamera, OitMaterial, OitMaterialMeshBundle, OitPlugin};
use utils::{
    camera_controller::{CameraController, CameraControllerPlugin},
    gooch_material::GoochMaterial,
};

mod utils;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            MaterialPlugin::<GoochMaterial>::default(),
            CameraControllerPlugin,
            OitPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (update_scene_material, toggle_material))
        .run();
}

#[derive(Component)]
struct KeepMaterial;

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, asset_server: Res<AssetServer>) {
    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 1.0, 5.0),
            camera_3d: Camera3d {
                depth_texture_usages: (TextureUsages::RENDER_ATTACHMENT
                    | TextureUsages::TEXTURE_BINDING)
                    .into(),
                ..default()
            },
            ..default()
        },
        CameraController::default(),
        OitCamera,
    ));

    // Text
    commands.spawn(TextBundle::from_section(
        "OIT: On",
        TextStyle {
            font_size: 36.0,
            ..default()
        },
    ));

    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // dragon
    commands.spawn(SceneBundle {
        scene: asset_server.load("dragon.glb#Scene0"),
        ..default()
    });

    // Spheres
    let sphere_handle = meshes.add(UVSphere::default().into());
    commands.spawn(OitMaterialMeshBundle {
        mesh: sphere_handle.clone(),
        material: OitMaterial {
            base_color: Color::RED.with_a(0.75),
        },
        transform: Transform::from_xyz(-1., 0., 0.),
        ..default()
    });
    commands.spawn(OitMaterialMeshBundle {
        mesh: sphere_handle.clone(),
        material: OitMaterial {
            base_color: Color::RED.with_a(0.5),
        },
        transform: Transform::from_xyz(0., 0., 0.),
        ..default()
    });
    commands.spawn(OitMaterialMeshBundle {
        mesh: sphere_handle.clone(),
        material: OitMaterial {
            base_color: Color::RED.with_a(0.1),
        },
        transform: Transform::from_xyz(1., 0., 0.),
        ..default()
    });
}

fn update_scene_material(
    mut commands: Commands,
    q: Query<Entity, (With<Handle<StandardMaterial>>, Without<KeepMaterial>)>,
) {
    for e in &q {
        commands
            .entity(e)
            .remove::<Handle<StandardMaterial>>()
            .insert(OitMaterial {
                base_color: Color::WHITE.with_a(0.5),
            });
    }
}

#[allow(clippy::type_complexity)]
fn toggle_material(
    mut commands: Commands,
    q: Query<(Entity, Option<&Handle<GoochMaterial>>, Option<&OitMaterial>)>,
    keyboard_input: Res<Input<KeyCode>>,
    mut materials: ResMut<Assets<GoochMaterial>>,
    mut text: Query<&mut Text>,
    mut oit_enabled: Local<bool>,
) {
    if !keyboard_input.just_pressed(KeyCode::Space) {
        return;
    }

    if *oit_enabled {
        text.single_mut().sections[0].value = "OIT: On".into();
    } else {
        text.single_mut().sections[0].value = "OIT: Off".into();
    }
    *oit_enabled = !*oit_enabled;

    for (e, gooch, oit) in &q {
        if let Some(handle) = gooch {
            if let Some(gooch) = materials.get(handle) {
                commands
                    .entity(e)
                    .remove::<Handle<GoochMaterial>>()
                    .insert(OitMaterial {
                        base_color: gooch.base_color,
                    });
            }
        } else if let Some(oit) = oit {
            commands
                .entity(e)
                .remove::<OitMaterial>()
                .insert(materials.add(GoochMaterial {
                    base_color: oit.base_color,
                    ..default()
                }));
        }
    }
}
