use crate::{Shader, Vertex2d};
use hal::{
    command::{CommandBuffer, Shot},
    format::Format,
    pass::Subpass,
    pso::{
        AttributeDesc, BlendState, ColorBlendDesc, ColorMask, Element, GraphicsPipelineDesc,
        GraphicsShaderSet, Rasterizer, Specialization, SpecializationConstant, VertexBufferDesc,
        VertexInputRate,
    },
    queue::capability::{Graphics, Supports},
    Primitive,
};

pub struct ShaderSet<'a, D, B>
where
    D: hal::Device<B>,
    B: hal::Backend,
{
    vertex_shader: Shader<'a, D, B>,
    fragment_shader: Option<Shader<'a, D, B>>,
}

impl<'a, D, B> ShaderSet<'a, D, B>
where
    D: hal::Device<B>,
    B: hal::Backend,
{
    /// Construct a new shader group using the specified vertex shader.
    pub fn new(vertex_shader: Shader<'a, D, B>) -> Self {
        ShaderSet {
            vertex_shader,
            fragment_shader: None,
        }
    }

    /// Associate a fragment shader with the group.
    pub fn fragment_shader(mut self, fragment_shader: Shader<'a, D, B>) -> Self {
        self.fragment_shader = Some(fragment_shader);
        self
    }

    /// Convert into a graphics shader set.
    fn to_graphics_shader_set(&self) -> GraphicsShaderSet<'_, B> {
        let mut vertex = self.vertex_shader.entry_point();

        vertex.specialization = Specialization {
            constants: &[SpecializationConstant { id: 0, range: 0..4 }],
            data: unsafe { std::mem::transmute::<&f32, &[u8; 4]>(&0.8f32) },
        };

        let fragment = self.fragment_shader.as_ref().map(|fs| fs.entry_point());

        GraphicsShaderSet {
            vertex,
            hull: None,
            domain: None,
            geometry: None,
            fragment,
        }
    }
}

/// A graphics pipeline describing how to do graphics.
pub struct GraphicsPipeline<'a, D, B>
where
    D: hal::Device<B>,
    B: hal::Backend,
{
    device: &'a D,
    graphics_pipeline: B::GraphicsPipeline,
}

impl<'a, D, B> GraphicsPipeline<'a, D, B>
where
    D: hal::Device<B>,
    B: hal::Backend,
{
    /// Create a new graphics pipeline.
    pub fn new(
        device: &'a D,
        render_pass: &B::RenderPass,
        pipeline_layout: &B::PipelineLayout,
        shader_set: ShaderSet<'a, D, B>,
    ) -> Result<Self, failure::Error>
    where
        D: hal::Device<B>,
    {
        let shader_set = shader_set.to_graphics_shader_set();

        let subpass = Subpass {
            index: 0,
            main_pass: render_pass,
        };

        let mut pipeline_desc = GraphicsPipelineDesc::new(
            shader_set,
            Primitive::TriangleList,
            Rasterizer::FILL,
            pipeline_layout,
            subpass,
        );

        pipeline_desc
            .blender
            .targets
            .push(ColorBlendDesc(ColorMask::ALL, BlendState::ALPHA));

        pipeline_desc.vertex_buffers.push(VertexBufferDesc {
            binding: 0,
            stride: std::mem::size_of::<Vertex2d>() as u32,
            rate: VertexInputRate::Vertex,
        });

        pipeline_desc.attributes.push(AttributeDesc {
            location: 0,
            binding: 0,
            element: Element {
                format: Format::Rg32Sfloat,
                offset: 0,
            },
        });
        pipeline_desc.attributes.push(AttributeDesc {
            location: 1,
            binding: 0,
            element: Element {
                format: Format::Rg32Sfloat,
                offset: 8,
            },
        });

        let graphics_pipeline = unsafe { device.create_graphics_pipeline(&pipeline_desc, None) }?;
        Ok(GraphicsPipeline {
            device,
            graphics_pipeline,
        })
    }

    /// Bind to command buffer.
    pub fn bind<C, S>(&self, buffer: &mut CommandBuffer<B, C, S>)
    where
        S: Shot,
        C: Supports<Graphics>,
    {
        unsafe {
            buffer.bind_graphics_pipeline(&self.graphics_pipeline);
        }
    }
}

impl<'a, D, B> Drop for GraphicsPipeline<'a, D, B>
where
    D: hal::Device<B>,
    B: hal::Backend,
{
    fn drop(&mut self) {
        use std::mem;

        unsafe {
            self.device.destroy_graphics_pipeline(mem::replace(
                &mut self.graphics_pipeline,
                mem::zeroed(),
            ));
        }
    }
}
