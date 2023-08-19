use bevy::{
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prelude::*,
    render::{
        render_resource::{
            encase::private::WriteInto, BindGroup, BindGroupDescriptor, BindGroupEntry,
            BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
            BindingType, BlendState, BufferBinding, ColorTargetState, ColorWrites,
            DepthStencilState, DynamicUniformBuffer, FragmentState, MultisampleState,
            PrimitiveState, RenderPipelineDescriptor, ShaderDefVal, ShaderStages, ShaderType,
            StorageBuffer, TextureFormat, TextureView, UniformBuffer, VertexBufferLayout,
            VertexState,
        },
        renderer::RenderDevice,
        texture::BevyDefault,
    },
};

#[allow(unused)]
pub fn color_target(blend: Option<BlendState>) -> ColorTargetState {
    ColorTargetState {
        format: TextureFormat::bevy_default(),
        blend,
        write_mask: ColorWrites::ALL,
    }
}

pub fn fragment_state(
    shader: Handle<Shader>,
    entry_point: &'static str,
    targets: &[ColorTargetState],
    shader_defs: &[ShaderDefVal],
) -> Option<FragmentState> {
    Some(FragmentState {
        entry_point: entry_point.into(),
        shader,
        shader_defs: shader_defs.to_vec(),
        targets: targets.iter().map(|target| Some(target.clone())).collect(),
    })
}

pub fn vertex_state(
    shader: Handle<Shader>,
    entry_point: &'static str,
    shader_defs: &[ShaderDefVal],
    buffers: &[VertexBufferLayout],
) -> VertexState {
    VertexState {
        entry_point: entry_point.into(),
        shader,
        shader_defs: shader_defs.to_vec(),
        buffers: buffers.to_vec(),
    }
}

pub trait RenderDeviceExt {
    fn create_bind_group_ext<const S: usize>(
        &self,
        label: &'static str,
        layout: &BindGroupLayout,
        entries: [BindGroupEntry; S],
    ) -> BindGroup;
    fn create_bind_group_layout_ext<const S: usize>(
        &self,
        label: &'static str,
        entries: [(ShaderStages, BindingType); S],
    ) -> BindGroupLayout;
}

impl RenderDeviceExt for RenderDevice {
    #[inline]
    fn create_bind_group_ext<const S: usize>(
        &self,
        label: &'static str,
        layout: &BindGroupLayout,
        mut entries: [BindGroupEntry; S],
    ) -> BindGroup {
        let mut auto = false;
        for (index, entry) in entries.iter_mut().enumerate() {
            if entry.binding == u32::MAX {
                entry.binding = index as u32;
                auto = true;
            } else if auto {
                panic!("Cannot mix manual binding indices with automatic indices");
            }
        }
        self.create_bind_group(&BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &entries,
        })
    }

    fn create_bind_group_layout_ext<const S: usize>(
        &self,
        label: &'static str,
        entries: [(ShaderStages, BindingType); S],
    ) -> BindGroupLayout {
        let entries = entries
            .iter()
            .enumerate()
            .map(|(i, (vis, ty))| BindGroupLayoutEntry {
                binding: i as u32,
                visibility: *vis,
                ty: *ty,
                count: None,
            })
            .collect::<Vec<_>>();
        self.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(label),
            entries: &entries,
        })
    }
}

pub trait BindingResouceExt {
    fn binding_index(&self, binding_index: u32) -> BindGroupEntry;
    fn binding_entry(&self) -> BindGroupEntry;
}
impl<T: ShaderType + WriteInto> BindingResouceExt for UniformBuffer<T> {
    #[inline]
    fn binding_index(&self, binding_index: u32) -> BindGroupEntry {
        BindGroupEntry {
            binding: binding_index,
            resource: BindingResource::Buffer(
                self.buffer()
                    .expect("Failed to get buffer")
                    .as_entire_buffer_binding(),
            ),
        }
    }
    fn binding_entry(&self) -> BindGroupEntry {
        self.binding_index(u32::MAX)
    }
}
impl<T: ShaderType + WriteInto> BindingResouceExt for StorageBuffer<T> {
    #[inline]
    fn binding_index(&self, binding_index: u32) -> BindGroupEntry {
        BindGroupEntry {
            binding: binding_index,
            resource: BindingResource::Buffer(
                self.buffer()
                    .expect("Failed to get buffer")
                    .as_entire_buffer_binding(),
            ),
        }
    }
    fn binding_entry(&self) -> BindGroupEntry {
        self.binding_index(u32::MAX)
    }
}
impl BindingResouceExt for TextureView {
    #[inline]
    fn binding_index(&self, binding_index: u32) -> BindGroupEntry {
        BindGroupEntry {
            binding: binding_index,
            resource: BindingResource::TextureView(self),
        }
    }

    #[inline]
    fn binding_entry(&self) -> BindGroupEntry {
        self.binding_index(u32::MAX)
    }
}
impl<T: ShaderType + WriteInto> BindingResouceExt for DynamicUniformBuffer<T> {
    #[inline]
    fn binding_index(&self, binding_index: u32) -> BindGroupEntry {
        BindGroupEntry {
            binding: binding_index,
            resource: BindingResource::Buffer(BufferBinding {
                buffer: self.buffer().expect("Failed to get buffer"),
                offset: 0,
                size: Some(T::min_size()),
            }),
        }
    }

    #[inline]
    fn binding_entry(&self) -> BindGroupEntry {
        self.binding_index(u32::MAX)
    }
}

pub struct RenderPipelineDescriptorBuilder {
    desc: RenderPipelineDescriptor,
}

impl RenderPipelineDescriptorBuilder {
    #[allow(unused)]
    pub fn new(vertex_state: VertexState) -> RenderPipelineDescriptorBuilder {
        Self {
            desc: RenderPipelineDescriptor {
                fragment: None,
                vertex: vertex_state,
                label: None,
                layout: vec![],
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                push_constant_ranges: vec![],
            },
        }
    }

    pub fn fullscreen() -> RenderPipelineDescriptorBuilder {
        Self {
            desc: RenderPipelineDescriptor {
                fragment: None,
                vertex: fullscreen_shader_vertex_state(),
                label: None,
                layout: vec![],
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                push_constant_ranges: vec![],
            },
        }
    }

    pub fn label(mut self, label: &'static str) -> Self {
        self.desc.label = Some(label.into());
        self
    }

    pub fn fragment(
        mut self,
        shader: Handle<Shader>,
        entry_point: &'static str,
        targets: &[ColorTargetState],
        shader_defs: &[ShaderDefVal],
    ) -> Self {
        self.desc.fragment = fragment_state(shader, entry_point, targets, shader_defs);
        self
    }

    #[allow(unused)]
    pub fn vertex(
        mut self,
        shader: Handle<Shader>,
        entry_point: &'static str,
        shader_defs: &[ShaderDefVal],
    ) -> Self {
        self.desc.vertex = vertex_state(shader, entry_point, shader_defs, &[]);
        self
    }

    pub fn layout(mut self, layouts: Vec<BindGroupLayout>) -> Self {
        self.desc.layout = layouts;
        self
    }

    #[allow(unused)]
    pub fn depth_stencil(mut self, state: DepthStencilState) -> Self {
        self.desc.depth_stencil = Some(state);
        self
    }

    pub fn build(self) -> RenderPipelineDescriptor {
        self.desc
    }
}
