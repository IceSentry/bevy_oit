use std::{fs::File, io::BufReader, iter::Peekable, slice::Iter};

use bevy::{
    math::DVec3,
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::{
        mesh::{Indices, VertexAttributeValues},
        render_resource::{AsBindGroup, PrimitiveTopology, ShaderRef, TextureUsages},
    },
    window::{PresentMode, WindowResolution},
};
use bevy_oit::{OitCamera, OitMaterial, OitMaterialMeshBundle, OitPlugin};
use camera_controller::{CameraController, CameraControllerPlugin};
use dxf::entities::EntityType;
use nalgebra::{Isometry3, Point3};
use rand::Rng;

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
                    // present_mode: PresentMode::AutoNoVsync,
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

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 200.0, 500.0),
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
