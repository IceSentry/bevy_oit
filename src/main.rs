use bevy::{
    prelude::{shape::UVSphere, *},
    reflect::{TypePath, TypeUuid},
    render::render_resource::{AsBindGroup, ShaderRef},
    window::WindowResolution,
};
use bevy_oit::custom_phase::{CustomMaterial, CustomMaterialMeshBundle, CustomRenderPhasePlugin};
use bevy_oit::oit_phase::{OitMaterial, OitMesh};
use camera_controller::{CameraController, CameraControllerPlugin};

mod camera_controller;

// TODO handle resize

fn main() {
    App::new()
        .insert_resource(Msaa::Off)
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(
                        bevy_oit::WINDOW_WIDTH as f32,
                        bevy_oit::WINDOW_HEIGHT as f32,
                    ),
                    ..default()
                }),
                ..default()
            }),
            MaterialPlugin::<GoochMaterial>::default(),
            // OitMeshPlugin,
            // OitPlugin,
            CameraControllerPlugin,
            CustomRenderPhasePlugin,
        ))
        .add_systems(Startup, setup)
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
            transform: Transform::from_xyz(0.0, 1.0, 5.0),
            ..default()
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

    let sphere_handle = meshes.add(UVSphere::default().into());

    // OIT
    commands.spawn(OitBundle {
        mesh: sphere_handle.clone(),
        material: OitMaterial {
            base_color: Color::RED.with_a(0.5),
        },
        transform: Transform::from_translation(pos_a - offset),
        ..default()
    });
    commands.spawn(OitBundle {
        mesh: sphere_handle.clone(),
        material: OitMaterial {
            base_color: Color::GREEN.with_a(0.5),
        },
        transform: Transform::from_translation(pos_b - offset),
        ..default()
    });
    commands.spawn(OitBundle {
        mesh: sphere_handle.clone(),
        material: OitMaterial {
            base_color: Color::BLUE.with_a(0.5),
        },
        transform: Transform::from_translation(pos_c - offset),
        ..default()
    });

    // custom material
    commands.spawn(CustomMaterialMeshBundle {
        mesh: sphere_handle.clone(),
        material: CustomMaterial {
            base_color: Color::RED.with_a(0.5),
        },
        transform: Transform::from_translation(pos_a - offset),
        ..default()
    });
    commands.spawn(CustomMaterialMeshBundle {
        mesh: sphere_handle.clone(),
        material: CustomMaterial {
            base_color: Color::GREEN.with_a(0.5),
        },
        transform: Transform::from_translation(pos_b - offset),
        ..default()
    });
    commands.spawn(CustomMaterialMeshBundle {
        mesh: sphere_handle.clone(),
        material: CustomMaterial {
            base_color: Color::BLUE.with_a(0.5),
        },
        transform: Transform::from_translation(pos_c - offset),
        ..default()
    });

    // Alpha Blend
    commands.spawn(MaterialMeshBundle {
        mesh: sphere_handle.clone(),
        material: materials.add(GoochMaterial {
            base_color: Color::RED.with_a(0.5),
        }),
        transform: Transform::from_translation(pos_a + offset),
        ..default()
    });
    commands.spawn(MaterialMeshBundle {
        mesh: sphere_handle.clone(),
        material: materials.add(GoochMaterial {
            base_color: Color::GREEN.with_a(0.5),
        }),
        transform: Transform::from_translation(pos_b + offset),
        ..default()
    });
    commands.spawn(MaterialMeshBundle {
        mesh: sphere_handle,
        material: materials.add(GoochMaterial {
            base_color: Color::BLUE.with_a(0.5),
        }),
        transform: Transform::from_translation(pos_c + offset),
        ..default()
    });
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
