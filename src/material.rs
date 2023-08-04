use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::{
        render_asset::{PrepareAssetError, RenderAsset},
        render_resource::{
            AsBindGroup, AsBindGroupError, BindGroup, BindGroupLayout, OwnedBindingResource,
            PreparedBindGroup, ShaderType,
        },
        renderer::RenderDevice,
    },
};

pub struct OitMaterialAssetPlugin;
impl Plugin for OitMaterialAssetPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<OitMaterialAsset>();
    }
}

#[derive(TypeUuid, TypePath, Debug, Clone, ShaderType)]
#[uuid = "eb8e4d86-5e76-57cd-9eb3-00a2ad641233"]
pub struct OitMaterialAsset {
    base_color: Color,
}

impl AsBindGroup for OitMaterialAsset {
    type Data = Self;

    fn as_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        images: &bevy::render::render_asset::RenderAssets<Image>,
        fallback_image: &bevy::render::texture::FallbackImage,
    ) -> Result<PreparedBindGroup<Self::Data>, AsBindGroupError> {
        todo!()
    }

    fn bind_group_layout(
        render_device: &RenderDevice,
    ) -> bevy::render::render_resource::BindGroupLayout
    where
        Self: Sized,
    {
        todo!()
    }
}

// struct PreparedOitMaterial {
//     pub bindings: Vec<OwnedBindingResource>,
//     pub bind_group: BindGroup,
// }

// impl RenderAsset for OitMaterialAsset {
//     type ExtractedAsset = Self;

//     type PreparedAsset = Self;

//     type Param = SRes<RenderDevice>;

//     fn extract_asset(&self) -> Self::ExtractedAsset {
//         self.clone()
//     }

//     fn prepare_asset(
//         extracted_asset: Self::ExtractedAsset,
//         param: &mut SystemParamItem<Self::Param>,
//     ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
//         todo!()
//     }
// }
