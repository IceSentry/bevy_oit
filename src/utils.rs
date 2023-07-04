use bevy::{
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prelude::*,
    render::{
        render_resource::{
            BindGroupLayout, BlendState, ColorTargetState, ColorWrites, FragmentState,
            MultisampleState, PrimitiveState, RenderPipelineDescriptor, ShaderDefVal,
            TextureFormat, VertexBufferLayout, VertexState,
        },
        texture::BevyDefault,
    },
};

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

#[macro_export]
macro_rules! bind_group_entries {
    ($($index:expr => $res:expr,)*) => {
        [$(
            bevy::render::render_resource::BindGroupEntry {
                binding: $index,
                resource: $res,
            },
        )*]
    };
}

#[macro_export]
macro_rules! bind_group_layout_entries {
    ($($index:expr => ($vis:expr, $ty:expr),)*) => {
        [$(
            bevy::render::render_resource::BindGroupLayoutEntry {
                binding: $index,
                ty: $ty,
                visibility: $vis,
                count: None
            },
        )*]
    };
}

pub struct RenderPipelineDescriptorBuilder {
    desc: RenderPipelineDescriptor,
}

impl RenderPipelineDescriptorBuilder {
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

    pub fn build(self) -> RenderPipelineDescriptor {
        self.desc
    }
}

pub mod view_node {
    use bevy::{
        ecs::query::{QueryItem, ReadOnlyWorldQuery},
        prelude::*,
        render::{
            render_graph::{NodeRunError, RenderGraphContext, SlotInfo, SlotType},
            renderer::RenderContext,
        },
    };

    pub trait ViewNode {
        type ViewQuery: ReadOnlyWorldQuery;

        fn update(&mut self, _world: &mut World) {}

        fn run(
            &self,
            graph: &mut RenderGraphContext,
            render_context: &mut RenderContext,
            view_query: QueryItem<Self::ViewQuery>,
            world: &World,
        ) -> Result<(), NodeRunError>;
    }

    pub struct ViewNodeRunner<N: ViewNode> {
        view_query: QueryState<N::ViewQuery>,
        node: N,
    }

    impl<N: ViewNode> ViewNodeRunner<N> {
        const IN_VIEW: &str = "view";

        pub fn new(node: N, world: &mut World) -> Self {
            Self {
                view_query: world.query_filtered(),
                node,
            }
        }
    }

    impl<N: ViewNode + FromWorld> FromWorld for ViewNodeRunner<N> {
        fn from_world(world: &mut World) -> Self {
            Self::new(N::from_world(world), world)
        }
    }

    impl<T> bevy::render::render_graph::Node for ViewNodeRunner<T>
    where
        T: ViewNode + Send + Sync + 'static,
    {
        fn input(&self) -> Vec<SlotInfo> {
            vec![SlotInfo::new(Self::IN_VIEW, SlotType::Entity)]
        }

        fn update(&mut self, world: &mut World) {
            self.view_query.update_archetypes(world);
            self.node.update(world);
        }

        fn run(
            &self,
            graph: &mut RenderGraphContext,
            render_context: &mut RenderContext,
            world: &World,
        ) -> Result<(), NodeRunError> {
            let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
            let Ok(view) = self
                .view_query
                .get_manual(world, view_entity)
            else {
                return Ok(());
            };

            ViewNode::run(&self.node, graph, render_context, view, world)?;
            Ok(())
        }
    }
}

mod render_graph_app {
    use bevy::{
        prelude::*,
        render::render_graph::{Node, RenderGraph, RenderGraphError},
    };

    pub trait RenderGraphApp {
        fn add_render_sub_graph(&mut self, sub_graph_name: &'static str) -> &mut Self;
        fn add_render_graph_node<T: Node + FromWorld>(
            &mut self,
            sub_graph_name: &'static str,
            node_name: &'static str,
        ) -> &mut Self;
        fn add_render_graph_edges(
            &mut self,
            sub_graph_name: &'static str,
            edges: &[&'static str],
        ) -> &mut Self;
        fn add_render_graph_edge(
            &mut self,
            sub_graph_name: &'static str,
            output_edge: &'static str,
            input_edge: &'static str,
        ) -> &mut Self;
    }

    impl RenderGraphApp for App {
        fn add_render_graph_node<T: Node + FromWorld>(
            &mut self,
            sub_graph_name: &'static str,
            node_name: &'static str,
        ) -> &mut Self {
            let node = T::from_world(&mut self.world);
            let mut render_graph = self.world.get_resource_mut::<RenderGraph>().expect(
            "RenderGraph not found. Make sure you are using add_render_graph_node on the RenderApp",
        );
            if let Some(graph) = render_graph.get_sub_graph_mut(sub_graph_name) {
                graph.add_node(node_name, node);
            } else {
                warn!("Tried adding a render graph node to {sub_graph_name} but the sub graph doesn't exist");
            }
            self
        }

        fn add_render_graph_edges(
            &mut self,
            sub_graph_name: &'static str,
            edges: &[&'static str],
        ) -> &mut Self {
            let mut render_graph = self.world.get_resource_mut::<RenderGraph>().expect(
            "RenderGraph not found. Make sure you are using add_render_graph_edges on the RenderApp",
        );
            if let Some(graph) = render_graph.get_sub_graph_mut(sub_graph_name) {
                add_node_edges(graph, edges);
            } else {
                warn!("Tried adding render graph edges to {sub_graph_name} but the sub graph doesn't exist");
            }
            self
        }

        fn add_render_graph_edge(
            &mut self,
            sub_graph_name: &'static str,
            output_edge: &'static str,
            input_edge: &'static str,
        ) -> &mut Self {
            let mut render_graph = self.world.get_resource_mut::<RenderGraph>().expect(
            "RenderGraph not found. Make sure you are using add_render_graph_edge on the RenderApp",
        );
            if let Some(graph) = render_graph.get_sub_graph_mut(sub_graph_name) {
                graph.add_node_edge(output_edge, input_edge);
            } else {
                warn!("Tried adding a render graph edge to {sub_graph_name} but the sub graph doesn't exist");
            }
            self
        }

        fn add_render_sub_graph(&mut self, sub_graph_name: &'static str) -> &mut Self {
            let mut render_graph = self.world.get_resource_mut::<RenderGraph>().expect(
                "RenderGraph not found. Make sure you are using add_render_sub_graph on the RenderApp",
            );
            render_graph.add_sub_graph(sub_graph_name, RenderGraph::default());
            self
        }
    }

    /// Add `node_edge`s based on the order of the given `edges` array.
    ///
    /// Defining an edge that already exists is not considered an error with this api.
    /// It simply won't create a new edge.
    pub fn add_node_edges(render_graph: &mut RenderGraph, edges: &[&'static str]) {
        for window in edges.windows(2) {
            let [a, b] = window else { break; };
            if let Err(err) = render_graph.try_add_node_edge(*a, *b) {
                match err {
                    // Already existing edges are very easy to produce with this api
                    // and shouldn't cause a panic
                    RenderGraphError::EdgeAlreadyExists(_) => {}
                    _ => panic!("{err:?}"),
                }
            }
        }
    }
}
