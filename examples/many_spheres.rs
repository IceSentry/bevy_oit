use bevy::{
    prelude::{shape::UVSphere, *},
    render::render_resource::TextureUsages,
    window::PresentMode,
};
use bevy_oit::{
    material::{OitMaterial, OitMaterialMeshBundle},
    OitCamera, OitPlugin,
};
use rand::Rng;
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
        .add_systems(Update, toggle_material)
        .run();
}

#[derive(Component)]
struct KeepMaterial;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut oit_materials: ResMut<Assets<OitMaterial>>,
) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 30.0),
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

    // Text
    commands.spawn(TextBundle::from_section(
        "OIT: On",
        TextStyle {
            font_size: 36.0,
            ..default()
        },
    ));

    let sphere_handle = meshes.add(UVSphere::default().into());
    let mut spheres = vec![];
    let offset = 1.5;
    let mut rng = rand::thread_rng();
    let size = 10;
    for x in 0..=size {
        for y in 0..=size {
            for z in 0..=size {
                spheres.push(OitMaterialMeshBundle {
                    mesh: sphere_handle.clone(),
                    material: oit_materials.add(OitMaterial {
                        base_color: Color::rgba(
                            rng.gen_range(0.0..1.0),
                            rng.gen_range(0.0..1.0),
                            rng.gen_range(0.0..1.0),
                            0.5,
                        ),
                    }),
                    transform: Transform::from_xyz(
                        (x as f32 - size as f32 / 2.0) * offset,
                        (y as f32 - size as f32 / 2.0) * offset,
                        (z as f32 - size as f32 / 2.0) * offset,
                    ),
                    ..default()
                });
            }
        }
    }
    commands.spawn_batch(spheres);
}

#[allow(clippy::type_complexity)]
fn toggle_material(
    mut commands: Commands,
    q: Query<(
        Entity,
        Option<&Handle<GoochMaterial>>,
        Option<&Handle<OitMaterial>>,
    )>,
    keyboard_input: Res<Input<KeyCode>>,
    mut gooch_materials: ResMut<Assets<GoochMaterial>>,
    mut oit_materials: ResMut<Assets<OitMaterial>>,
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
            if let Some(gooch) = gooch_materials.get(handle) {
                commands
                    .entity(e)
                    .remove::<Handle<GoochMaterial>>()
                    .insert(oit_materials.add(OitMaterial {
                        base_color: gooch.base_color,
                    }));
            }
        } else if let Some(handle) = oit {
            if let Some(oit) = oit_materials.get(handle) {
                commands
                    .entity(e)
                    .remove::<Handle<OitMaterial>>()
                    .insert(gooch_materials.add(GoochMaterial {
                        base_color: oit.base_color,
                        depth_bias: 0.0,
                    }));
            }
        }
    }
}
