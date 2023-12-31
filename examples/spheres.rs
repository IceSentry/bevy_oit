use bevy::{
    prelude::{shape::UVSphere, *},
    render::render_resource::TextureUsages,
    window::PresentMode,
};
use bevy_oit::{
    material::{OitMaterial, OitMaterialMeshBundle},
    OitCamera, OitPlugin,
};
use utils::{
    camera_controller::{CameraController, CameraControllerPlugin},
    gooch_material::GoochMaterial,
};

mod utils;

fn main() {
    App::new()
        .insert_resource(Msaa::Sample2)
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
        .run();
}

#[derive(Component)]
struct KeepMaterial;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GoochMaterial>>,
    mut oit_materials: ResMut<Assets<OitMaterial>>,
) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 5.0),
            camera_3d: Camera3d {
                depth_texture_usages: (TextureUsages::RENDER_ATTACHMENT
                    | TextureUsages::TEXTURE_BINDING)
                    .into(),
                ..default()
            },
            ..default()
        },
        CameraController::default(),
        OitCamera::default(),
    ));

    let pos_a = Vec3::new(-0.5, 0.25, 0.0);
    let pos_b = Vec3::new(0.0, -0.25, 0.0);
    let pos_c = Vec3::new(0.5, 0.25, 0.0);

    let offset = Vec3::new(1.65, 0.0, 0.0);

    let sphere_handle = meshes.add(UVSphere::default().into());

    let alpha = 0.5;

    // oit material
    commands.spawn(OitMaterialMeshBundle {
        mesh: sphere_handle.clone(),
        material: oit_materials.add(OitMaterial {
            base_color: Color::RED.with_a(alpha),
        }),
        transform: Transform::from_translation(pos_a - offset),
        ..default()
    });
    commands.spawn(OitMaterialMeshBundle {
        mesh: sphere_handle.clone(),
        material: oit_materials.add(OitMaterial {
            base_color: Color::GREEN.with_a(alpha),
        }),
        transform: Transform::from_translation(pos_b - offset),
        ..default()
    });
    commands.spawn(OitMaterialMeshBundle {
        mesh: sphere_handle.clone(),
        material: oit_materials.add(OitMaterial {
            base_color: Color::BLUE.with_a(alpha),
        }),
        transform: Transform::from_translation(pos_c - offset),
        ..default()
    });

    // Alpha Blend
    commands.spawn(MaterialMeshBundle {
        mesh: sphere_handle.clone(),
        material: materials.add(GoochMaterial {
            base_color: Color::RED.with_a(alpha),
            ..default()
        }),
        transform: Transform::from_translation(pos_a + offset),
        ..default()
    });
    commands.spawn(MaterialMeshBundle {
        mesh: sphere_handle.clone(),
        material: materials.add(GoochMaterial {
            base_color: Color::GREEN.with_a(alpha),
            ..default()
        }),
        transform: Transform::from_translation(pos_b + offset),
        ..default()
    });
    commands.spawn(MaterialMeshBundle {
        mesh: sphere_handle,
        material: materials.add(GoochMaterial {
            base_color: Color::BLUE.with_a(alpha),
            ..default()
        }),
        transform: Transform::from_translation(pos_c + offset),
        ..default()
    });
}
