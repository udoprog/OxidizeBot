use failure::format_err;
use hal::{
    adapter::MemoryProperties,
    buffer::Usage,
    command::{CommandBuffer, Shot},
    memory::Properties,
    queue::capability::{Graphics, Supports},
};

#[derive(Debug, Clone, Copy)]
#[allow(non_snake_case)]
pub struct Vertex2d {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
}

#[derive(Debug)]
pub struct VertexBuffer<'a, D, B>
where
    D: hal::Device<B>,
    B: hal::Backend,
{
    device: &'a D,
    vertex_buffer: B::Buffer,
    buffer_memory: B::Memory,
}

impl<'a, D, B> VertexBuffer<'a, D, B>
where
    D: hal::Device<B>,
    B: hal::Backend,
{
    pub fn load_2d(
        device: &'a D,
        vertices: &[Vertex2d],
        memory: &MemoryProperties,
    ) -> Result<VertexBuffer<'a, D, B>, failure::Error> {
        let buffer_stride = std::mem::size_of::<Vertex2d>() as u64;
        let buffer_len = vertices.len() as u64 * buffer_stride;

        if buffer_len == 0 {
            failure::bail!("cannot load zero-length buffers");
        }

        let mut vertex_buffer = unsafe { device.create_buffer(buffer_len, Usage::VERTEX) }?;

        let buffer_req = unsafe { device.get_buffer_requirements(&vertex_buffer) };

        let upload_type = memory
            .memory_types
            .iter()
            .enumerate()
            .position(|(id, mem_type)| {
                buffer_req.type_mask & (1 << id) != 0
                    && mem_type.properties.contains(Properties::CPU_VISIBLE)
            })
            .ok_or_else(|| format_err!("could not find suitable memory type"))?
            .into();

        let buffer_memory = unsafe { device.allocate_memory(upload_type, buffer_req.size) }?;

        unsafe { device.bind_buffer_memory(&buffer_memory, 0, &mut vertex_buffer) }?;

        // TODO: check transitions: read/write mapping and vertex buffer read
        unsafe {
            let mut writer =
                device.acquire_mapping_writer::<Vertex2d>(&buffer_memory, 0..buffer_req.size)?;
            writer[0..vertices.len()].copy_from_slice(vertices);
            device.release_mapping_writer(writer)?;
        }

        Ok(VertexBuffer {
            device,
            vertex_buffer,
            buffer_memory,
        })
    }

    /// Bind vertex buffers to command buffer.
    pub fn bind<C, S>(
        &self,
        buffer: &mut CommandBuffer<B, C, S>,
        first_binding: u32,
        second_binding: u64,
    ) where
        S: Shot,
        C: Supports<Graphics>,
    {
        unsafe {
            buffer.bind_vertex_buffers(first_binding, Some((&self.vertex_buffer, second_binding)));
        }
    }
}

impl<'a, D, B> Drop for VertexBuffer<'a, D, B>
where
    D: hal::Device<B>,
    B: hal::Backend,
{
    fn drop(&mut self) {
        use std::mem;

        unsafe {
            self.device
                .destroy_buffer(mem::replace(&mut self.vertex_buffer, mem::zeroed()));
            self.device
                .free_memory(mem::replace(&mut self.buffer_memory, mem::zeroed()));
        }
    }
}
