use std::{fs::File, io::BufReader, iter::Peekable, slice::Iter};

use bevy::{
    gizmos::GizmoPlugin,
    math::DVec3,
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::{
        mesh::{Indices, VertexAttributeValues},
        render_resource::{AsBindGroup, PrimitiveTopology, ShaderRef, TextureUsages},
    },
    window::WindowResolution,
};
use bevy_oit::{OitCamera, OitMaterial, OitMaterialMeshBundle, OitPlugin};
use camera_controller::{CameraController, CameraControllerPlugin};
use dxf::entities::EntityType;
use nalgebra::{Isometry3, Point3};
use rand::Rng;
use skybox::{Skybox, SkyboxPlugin, SkyboxSettings};

mod camera_controller;

fn main() {
    App::new()
        .insert_resource(Msaa::Off)
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(
                        bevy_oit::WINDOW_WIDTH as f32,
                        bevy_oit::WINDOW_HEIGHT as f32,
                    )
                    .with_scale_factor_override(1.0),
                    ..default()
                }),
                ..default()
            }),
            MaterialPlugin::<GoochMaterial>::default(),
            CameraControllerPlugin,
            OitPlugin,
            SkyboxPlugin,
        ))
        .insert_resource(SkyboxSettings::DARK_GRAY)
        .add_systems(Startup, setup)
        .add_systems(Update, toggle_material)
        .insert_resource(GizmoConfig {
            enabled: false,
            aabb: AabbGizmoConfig {
                draw_all: false,
                ..default()
            },
            ..default()
        })
        .run();
}

#[derive(Component)]
struct KeepMaterial;

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(250.0, 75.0, 300.0)
                .looking_at(Vec3::new(200.0, 0.0, 150.0), Vec3::Y),
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
        Skybox,
    ));

    commands.spawn(TextBundle::from_section(
        "OIT: On",
        TextStyle {
            font_size: 36.0,
            ..default()
        },
    ));

    let dxf_meshes = load_dxf("assets/Deswik_DXF.dxf").expect("Failed to load dxf");
    let mut rng = rand::thread_rng();
    for mesh in dxf_meshes {
        commands.spawn(OitMaterialMeshBundle {
            mesh: meshes.add(mesh),
            material: OitMaterial {
                base_color: Color::rgba(rng.gen(), rng.gen(), rng.gen(), 0.5),
            },
            transform: Transform::from_xyz(1., 0., 0.).with_rotation(Quat::from_euler(
                EulerRot::XYZ,
                -std::f32::consts::FRAC_PI_2,
                0.0,
                0.0,
            )),
            ..default()
        });
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
                }));
        }
    }
}

fn load_dxf(path: &str) -> anyhow::Result<Vec<Mesh>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let drawing = dxf::Drawing::load(&mut reader)?;
    let entities = drawing.entities().cloned().collect::<Vec<_>>();
    let mut entity_iter = entities.iter().peekable();

    let mut meshes = vec![];

    while let Some(entity) = entity_iter.next() {
        match &entity.specific {
            EntityType::Polyline(p) => {
                let mut vertices: Vec<DVec3> = vec![];
                let mut indices: Vec<[u32; 3]> = vec![];
                let index_shift = vertices.len() as i32;

                let mut optional_last_vertex = None;

                for (i, vertex) in p.vertices().enumerate() {
                    if i == 0 && p.get_is_closed() {
                        optional_last_vertex = Some(vertex.clone());
                    }
                    add_vertex(vertex, index_shift, &mut vertices, &mut indices);
                }

                if let Some(vertex) = optional_last_vertex {
                    add_vertex(&vertex, index_shift, &mut vertices, &mut indices);
                }

                if p.polygon_mesh_m_vertex_count == 0 {
                    // actual line so skip it
                } else {
                    let position =
                        Isometry3::translation(vertices[0][0], vertices[0][1], vertices[0][2]);
                    vertices.iter_mut().for_each(|point| {
                        let new_pos = position.inverse() * Point3::new(point.x, point.y, point.z);
                        *point = DVec3::new(new_pos.x, new_pos.y, new_pos.z);
                    });

                    let n_vertices = vertices.len();
                    let index_out_of_bound = indices
                        .iter()
                        .flatten()
                        .find(|index| **index as usize >= n_vertices);

                    if index_out_of_bound.is_some() {
                        anyhow::bail!("Load DXF: skipping polygon due to out-of-bounds index.");
                    }
                    meshes.push(build_bevy_mesh(&vertices, Some(&indices)));
                }
            }
            EntityType::Face3D(face) => {
                let mut vertices = vec![];
                let mut indices = vec![];
                let f1 = &face.first_corner;
                let origin = DVec3::new(f1.x, f1.y, f1.z);
                add_face_as_tris(face, origin, &mut vertices, &mut indices);
                while let Some(face) = try_next_entity_as_face(&mut entity_iter) {
                    add_face_as_tris(face, origin, &mut vertices, &mut indices);
                }
                meshes.push(build_bevy_mesh(&vertices, Some(&indices)));
            }
            EntityType::Text(_) => {
                continue;
            }
            x => {
                println!("unknown {:?}", x);
                // continue;
            }
        }
    }

    Ok(meshes)
}

/// Adds a dxf vertex to the supplied vert and index lists
pub fn add_vertex(
    v: &dxf::entities::Vertex,
    index_shift: i32,
    vertices: &mut Vec<DVec3>,
    indices: &mut Vec<[u32; 3]>,
) {
    match v.flags {
        128 => {
            // Face indices.
            indices.push([
                (v.polyface_mesh_vertex_index1 - 1 + index_shift) as u32,
                (v.polyface_mesh_vertex_index2 - 1 + index_shift) as u32,
                (v.polyface_mesh_vertex_index3 - 1 + index_shift) as u32,
            ]);

            if v.polyface_mesh_vertex_index4 != 0 {
                indices.push([
                    (v.polyface_mesh_vertex_index1 - 1 + index_shift) as u32,
                    (v.polyface_mesh_vertex_index3 - 1 + index_shift) as u32,
                    (v.polyface_mesh_vertex_index4 - 1 + index_shift) as u32,
                ]);
            }
        }
        192 | 32 => {
            vertices.push(DVec3::new(v.location.x, v.location.y, v.location.z));
        }
        _ => warn!("Found vertex with unhandled flags: {:?}", v),
    }
}

fn add_face_as_tris(
    face: &dxf::entities::Face3D,
    origin: DVec3,
    vertices: &mut Vec<DVec3>,
    indices: &mut Vec<[u32; 3]>,
) {
    let f1 = &face.first_corner;
    let f2 = &face.second_corner;
    let f3 = &face.third_corner;
    let f4 = &face.fourth_corner;
    let points = [
        DVec3::new(f1.x - origin.x, f1.y - origin.y, f1.z - origin.z),
        DVec3::new(f2.x - origin.x, f2.y - origin.y, f2.z - origin.z),
        DVec3::new(f3.x - origin.x, f3.y - origin.y, f3.z - origin.z),
        DVec3::new(f4.x - origin.x, f4.y - origin.y, f4.z - origin.z),
    ];

    let index_shift = vertices.len() as u32;
    add_point(&points, index_shift, vertices, indices);
}

fn add_point(p: &[DVec3; 4], i_shift: u32, vertices: &mut Vec<DVec3>, indices: &mut Vec<[u32; 3]>) {
    vertices.push(p[0]);
    vertices.push(p[1]);
    vertices.push(p[2]);
    indices.push([i_shift, i_shift + 1, i_shift + 2]);
    vertices.push(p[0]);
    vertices.push(p[2]);
    vertices.push(p[3]);
    indices.push([i_shift, i_shift + 2, i_shift + 3]);
}

fn try_next_entity_as_face<'a>(
    iter: &'a mut Peekable<Iter<'_, dxf::entities::Entity>>,
) -> Option<&'a dxf::entities::Face3D> {
    if let Some(entity) = iter.peek() {
        if let EntityType::Face3D(f) = &entity.specific {
            iter.next();
            return Some(f);
        }
    }
    None
}

pub fn build_bevy_mesh(vtx: &[DVec3], idx: Option<&[[u32; 3]]>) -> Mesh {
    let mut vertices = vec![];

    if let Some(idx) = idx {
        for idx in idx {
            let a = vtx[idx[0] as usize];
            let b = vtx[idx[1] as usize];
            let c = vtx[idx[2] as usize];

            vertices.push([a.x as f32, a.y as f32, a.z as f32]);
            vertices.push([b.x as f32, b.y as f32, b.z as f32]);
            vertices.push([c.x as f32, c.y as f32, c.z as f32]);

            vertices.push([a.x as f32, a.y as f32, a.z as f32]);
            vertices.push([c.x as f32, c.y as f32, c.z as f32]);
            vertices.push([b.x as f32, b.y as f32, b.z as f32]);
        }
    } else {
        vertices = vtx
            .iter()
            .map(|v| [v.x as f32, v.y as f32, v.z as f32])
            .collect();
    }
    println!("{:?}", vertices[0]);

    let indices: Vec<_> = (0..vertices.len() as u32).collect();

    // Compute basic normals.
    let mut normals = vec![];

    for vtx in vertices.chunks(3) {
        let a = Vec3::from_array(vtx[0]);
        let b = Vec3::from_array(vtx[1]);
        let c = Vec3::from_array(vtx[2]);
        let n = (b - a).cross(c - a).normalize();
        normals.push([n.x, n.y, n.z]);
        normals.push([n.x, n.y, n.z]);
        normals.push([n.x, n.y, n.z]);
    }

    normals
        .iter_mut()
        .for_each(|n| *n = Vec3::from(*n).normalize().into());

    // Dummy uvs.
    let uvs: Vec<_> = (0..vertices.len()).map(|_| [0.0, 0.0]).collect();

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::from(vertices),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, VertexAttributeValues::from(normals));
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, VertexAttributeValues::from(uvs));
    mesh.set_indices(Some(Indices::U32(indices)));
    mesh
}

mod skybox {
    use std::ops::RangeInclusive;

    use bevy::{
        prelude::*,
        render::{
            mesh::VertexAttributeValues,
            view::{Layer, NoFrustumCulling, RenderLayers},
        },
        transform::TransformSystem,
        utils::{FloatOrd, HashMap},
    };

    #[derive(Debug, Clone, Default)]
    pub struct SkyboxPlugin;
    impl Plugin for SkyboxPlugin {
        fn build(&self, app: &mut App) {
            app.add_systems(Startup, spawn_skybox)
                .add_systems(Update, update_skybox)
                .add_systems(
                    PostUpdate,
                    update_cameras.after(TransformSystem::TransformPropagate),
                );
        }
    }

    #[derive(Debug, Clone, Resource, PartialEq)]
    pub struct SkyboxSettings {
        pub sky: Color,
        pub ground: Color,
        /// The maximum range is `-1.0..1.0`: the gradient spans the entire height of the skybox. To
        /// push the gradient down, so the sky is a solid color and the transition starts at the
        /// horizon, use `-1.0..0.0`.
        pub gradient_range: RangeInclusive<f32>,
    }

    impl SkyboxSettings {
        pub const DARK_GRAY: Self = Self {
            sky: Color::rgb(0.37, 0.37, 0.37),
            ground: Color::rgb(0.2, 0.2, 0.2),
            gradient_range: -0.75..=0.30,
        };

        pub const DARK_BLUE: Self = Self {
            sky: Color::BLACK,
            ground: Color::rgb(0.1, 0.1, 0.3),
            gradient_range: -1.0..=-0.1,
        };

        pub const OVERCAST_SKY: Self = Self {
            sky: Color::rgb(0.55, 0.67, 0.94),
            ground: Color::rgb(0.78, 0.78, 0.78),
            gradient_range: -0.7..=0.0,
        };

        pub const SHERBERT: Self = Self {
            sky: Color::PINK,
            ground: Color::GOLD,
            gradient_range: -0.8..=0.8,
        };
    }

    impl Default for SkyboxSettings {
        fn default() -> Self {
            Self::DARK_GRAY
        }
    }

    pub const SKYBOX_RENDER_LAYER: Layer = 9;
    pub const SKYBOX_CAM_ORDER: isize = -10_000;

    /// Marker component to tag the skybox mesh
    #[derive(Debug, Default, Clone, Copy, Component)]
    pub struct SkyboxMesh;

    /// Marks a camera that should have a skybox rendered.
    #[derive(Debug, Default, Clone, Copy, Component)]
    pub struct Skybox;

    /// Marker component to tag camera used to render the skybox
    #[derive(Debug, Clone, Component)]
    pub struct SkyboxFollowCamera {
        following: Entity,
    }

    fn spawn_skybox(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        println!("spawn skybox");
        commands.spawn((
            SkyboxMesh,
            NoFrustumCulling,
            PbrBundle {
                mesh: meshes.add(
                    Mesh::try_from(shape::UVSphere {
                        sectors: 24,
                        stacks: 64,
                        radius: 1.0,
                    })
                    .unwrap(),
                ),
                material: materials.add(StandardMaterial {
                    cull_mode: Some(bevy::render::render_resource::Face::Front),
                    unlit: true,
                    depth_bias: -1e5_f32,
                    ..Color::WHITE.into() // vertex colors are multiplied by the base color
                }),
                transform: Transform::default()
                    .with_scale(Vec3::splat(1e5_f32))
                    .with_rotation(Quat::from_euler(
                        EulerRot::XYZ,
                        -std::f32::consts::FRAC_PI_2,
                        0.0,
                        0.0,
                    )),
                ..default()
            },
            // RenderLayers::layer(SKYBOX_RENDER_LAYER),
        ));
    }

    pub fn update_skybox(
        mut meshes: ResMut<Assets<Mesh>>,
        settings: Res<SkyboxSettings>,
        mut last_skybox_settings: Local<Option<SkyboxSettings>>,
        skybox: Query<&Handle<Mesh>, With<SkyboxMesh>>,
    ) {
        // Only continue if we haven't already made a skybox with these settings
        match last_skybox_settings.as_ref() {
            Some(x) if x != settings.as_ref() => {}
            None => {}
            _ => return,
        }

        let Some(skybox_mesh) = meshes.get_mut(skybox.single()) else {
            error!("failed to get sky mesh");
            return
        };

        println!("update skybox");

        if let Some(VertexAttributeValues::Float32x3(positions)) =
            skybox_mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        {
            let z_min = positions.iter().map(|p| FloatOrd(p[2])).min().map(|n| n.0);
            let z_min = z_min.unwrap_or_default() + settings.gradient_range.start() + 1.0;
            let z_max = positions.iter().map(|p| FloatOrd(p[2])).max().map(|n| n.0);
            let z_max = z_max.unwrap_or_default() + settings.gradient_range.end() - 1.0;

            let lerp = |s: f32| ((s - z_min) / (z_max - z_min)).clamp(0.0, 1.0);
            let mix = |s: f32, c1: Color, c2: Color| {
                let c1 = Vec4::from_array(c1.as_rgba_f32());
                let c2 = Vec4::from_array(c2.as_rgba_f32());
                let mixed = c1.lerp(c2, s);
                Color::from(mixed)
            };

            let colors: Vec<[f32; 4]> = positions
                .iter()
                .map(|[_x, _y, z]| {
                    mix(lerp(*z), settings.ground, settings.sky).as_linear_rgba_f32()
                })
                .collect();
            skybox_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);

            *last_skybox_settings = Some(settings.to_owned());
        }
    }

    pub fn update_cameras(
        mut commands: Commands,
        mut skybox_cameras: Query<(
            Entity,
            &SkyboxFollowCamera,
            &mut Camera,
            &mut Transform,
            &mut GlobalTransform,
        )>,
        cameras: Query<
            (Entity, &Camera, &GlobalTransform),
            (With<Skybox>, Without<SkyboxFollowCamera>),
        >,
    ) {
        let mut all_cameras: HashMap<_, _> =
            cameras.iter().map(|(e, c, gt)| (e, (c, gt))).collect();

        // Update existing skybox cameras
        for (entity, skybox_cam, mut camera, mut sky_cam_tfm, mut sky_cam_gtfm) in
            &mut skybox_cameras
        {
            if let Some((followed_cam, followed_cam_transform)) =
                all_cameras.remove(&skybox_cam.following)
            {
                // Update camera orientation
                let (_, rotation, _) = followed_cam_transform.to_scale_rotation_translation();
                *sky_cam_tfm = Transform {
                    rotation,
                    ..Transform::IDENTITY
                };
                *sky_cam_gtfm = sky_cam_tfm.compute_matrix().into();
                camera.target = followed_cam.target.clone();
                camera.viewport = followed_cam.viewport.clone();
                camera.is_active = followed_cam.is_active;
            } else {
                // The camera we are trying to follow doesn't exist anymore!
                commands.entity(entity).despawn_recursive();
            }
        }

        // If there are any cameras left in `all_cameras`, they need to have a skybox camera added.
        for (entity, (_, _)) in all_cameras.drain() {
            println!("spawn sky follow");
            commands.spawn((
                SkyboxFollowCamera { following: entity },
                Camera3dBundle {
                    tonemapping: bevy::core_pipeline::tonemapping::Tonemapping::None,
                    camera: Camera {
                        order: SKYBOX_CAM_ORDER,
                        ..default()
                    },
                    ..default()
                },
                RenderLayers::layer(SKYBOX_RENDER_LAYER),
            ));
        }
    }
}
