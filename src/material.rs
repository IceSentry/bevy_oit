use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::{
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_resource::{AsBindGroup, BindGroup, ShaderType},
        renderer::RenderDevice,
        texture::FallbackImage,
        Extract, RenderApp,
    },
};

use crate::pipeline::OitDrawPipeline;

pub struct OitMaterialPlugin;
impl Plugin for OitMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<OitMaterial>()
            .add_plugins(RenderAssetPlugin::<OitMaterial>::default());

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(ExtractSchedule, extract_oit_material);
    }
}

#[derive(TypeUuid, TypePath, Debug, Clone, ShaderType, AsBindGroup)]
#[uuid = "eb8e4d86-5e76-57cd-9eb3-00a2ad641233"]
pub struct OitMaterial {
    #[uniform(0)]
    pub base_color: Color,
}

#[derive(Bundle, Clone, Default)]
pub struct OitMaterialMeshBundle {
    pub mesh: Handle<Mesh>,
    pub material: Handle<OitMaterial>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
}

pub struct GpuOitMaterial {
    pub bind_group: BindGroup,
}

impl RenderAsset for OitMaterial {
    type ExtractedAsset = Self;

    type PreparedAsset = GpuOitMaterial;

    type Param = (
        SRes<RenderDevice>,
        SRes<OitDrawPipeline>,
        SRes<RenderAssets<Image>>,
        SRes<FallbackImage>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (render_device, pipeline, images, fallback_image): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let prepared_bind_group = extracted_asset
            .as_bind_group(
                &pipeline.oit_material_bind_group_layout,
                render_device,
                images,
                fallback_image,
            )
            .map_err(|_| PrepareAssetError::RetryNextUpdate(extracted_asset))?;
        Ok(GpuOitMaterial {
            bind_group: prepared_bind_group.bind_group,
        })
    }
}

fn extract_oit_material(
    mut commands: Commands,
    oit_materials: Extract<Query<(Entity, &Handle<OitMaterial>)>>,
) {
    for (entity, material) in &oit_materials {
        commands.get_or_spawn(entity).insert(material.clone());
    }
}
