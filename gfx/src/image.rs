use failure::format_err;
use failure::ResultExt;
use hal::{
    adapter::MemoryProperties,
    command::{BufferImageCopy, CommandBuffer, OneShot, Shot},
    format::{AsFormat, Aspects, Rgba8Srgb, Swizzle},
    image::{
        Access, Extent, Kind, Layout, Offset, Size, SubresourceLayers, SubresourceRange, Tiling,
        Usage, ViewCapabilities, ViewKind,
    },
    memory::{Barrier, Dependencies, Properties},
    pool::CommandPool,
    pso::{Descriptor, PipelineStage},
    queue::{
        capability::{Capability, Supports, Transfer},
        family::QueueGroup,
    },
};
use std::{fs::File, io::BufReader, path::Path};

const COLOR_RANGE: SubresourceRange = SubresourceRange {
    aspects: hal::format::Aspects::COLOR,
    levels: 0..1,
    layers: 0..1,
};

#[derive(Debug)]
pub struct Image<'a, D, B>
where
    D: hal::Device<B>,
    B: hal::Backend,
{
    device: &'a D,
    width: Size,
    height: Size,
    row_pitch: u32,
    image_stride: usize,
    image: B::Image,
    image_view: B::ImageView,
    image_upload_buffer: B::Buffer,
    image_memory: B::Memory,
    image_upload_memory: B::Memory,
}

impl<'a, D, B> Image<'a, D, B>
where
    D: hal::Device<B>,
    B: hal::Backend,
{
    pub fn load_png(
        device: &'a D,
        limits: hal::Limits,
        memory: &MemoryProperties,
        path: &Path,
    ) -> Result<Image<'a, D, B>, failure::Error>
    where
        B: hal::Backend,
    {
        let f = BufReader::new(File::open(path)?);

        let img = image::load(f, image::PNG)?.to_rgba();
        let (width, height) = img.dimensions();
        let kind = Kind::D2(width as Size, height as Size, 1, 1);
        let row_alignment_mask = limits.optimal_buffer_copy_pitch_alignment as u32 - 1;
        let image_stride = 4usize;
        let row_pitch = (width * image_stride as u32 + row_alignment_mask) & !row_alignment_mask;
        let upload_size = (height * row_pitch) as u64;

        let mut image_upload_buffer =
            unsafe { device.create_buffer(upload_size, hal::buffer::Usage::TRANSFER_SRC) }?;
        let image_mem_reqs = unsafe { device.get_buffer_requirements(&image_upload_buffer) };

        let upload_type = memory
            .memory_types
            .iter()
            .enumerate()
            .position(|(id, mem_type)| {
                image_mem_reqs.type_mask & (1 << id) != 0
                    && mem_type.properties.contains(Properties::CPU_VISIBLE)
            })
            .ok_or_else(|| format_err!("could not find suitable memory type"))?
            .into();

        let image_upload_memory =
            unsafe { device.allocate_memory(upload_type, image_mem_reqs.size) }?;

        unsafe { device.bind_buffer_memory(&image_upload_memory, 0, &mut image_upload_buffer) }?;

        // copy image data into staging buffer
        unsafe {
            let mut data = device
                .acquire_mapping_writer::<u8>(&image_upload_memory, 0..image_mem_reqs.size)?;
            for y in 0..height as usize {
                let row = &(*img)[y * (width as usize) * image_stride
                    ..(y + 1) * (width as usize) * image_stride];
                let dest_base = y * row_pitch as usize;
                data[dest_base..dest_base + row.len()].copy_from_slice(row);
            }
            device.release_mapping_writer(data)?;
        }

        let mut image = unsafe {
            device.create_image(
                kind,
                1,
                Rgba8Srgb::SELF,
                Tiling::Optimal,
                Usage::TRANSFER_DST | Usage::SAMPLED,
                ViewCapabilities::empty(),
            )
        }?;
        let image_req = unsafe { device.get_image_requirements(&image) };

        let device_type = memory
            .memory_types
            .iter()
            .enumerate()
            .position(|(id, memory_type)| {
                image_req.type_mask & (1 << id) != 0
                    && memory_type.properties.contains(Properties::DEVICE_LOCAL)
            })
            .ok_or_else(|| format_err!("could not find suitable device memory"))?
            .into();

        let image_memory = unsafe { device.allocate_memory(device_type, image_req.size) }?;

        unsafe { device.bind_image_memory(&image_memory, 0, &mut image) }?;

        let image_view = unsafe {
            device.create_image_view(
                &image,
                ViewKind::D2,
                Rgba8Srgb::SELF,
                Swizzle::NO,
                COLOR_RANGE.clone(),
            )
        }?;

        Ok(Image {
            device,
            width,
            height,
            row_pitch,
            image_stride,
            image,
            image_view,
            image_upload_buffer,
            image_memory,
            image_upload_memory,
        })
    }

    /// Construct a memory barrier for the image.
    pub fn as_barrier<'image>(
        &'image self,
        states: std::ops::Range<(Access, Layout)>,
    ) -> Barrier<'image, B> {
        Barrier::Image {
            states,
            target: &self.image,
            families: None,
            range: COLOR_RANGE.clone(),
        }
    }

    /// Treat as a descriptor.
    pub fn as_descriptor<'image>(&'image self) -> Descriptor<'image, B> {
        Descriptor::Image(&self.image_view, hal::image::Layout::Undefined)
    }

    /// Set up command buffer to copy upload buffer to image.
    pub fn copy_buffer_to_image<C, S>(&self, buffer: &mut CommandBuffer<B, C, S>)
    where
        S: Shot,
        C: Supports<Transfer>,
    {
        unsafe {
            buffer.copy_buffer_to_image(
                &self.image_upload_buffer,
                &self.image,
                hal::image::Layout::TransferDstOptimal,
                &[BufferImageCopy {
                    buffer_offset: 0,
                    buffer_width: self.row_pitch / (self.image_stride as u32),
                    buffer_height: self.height as u32,
                    image_layers: SubresourceLayers {
                        aspects: Aspects::COLOR,
                        level: 0,
                        layers: 0..1,
                    },
                    image_offset: Offset { x: 0, y: 0, z: 0 },
                    image_extent: Extent {
                        width: self.width,
                        height: self.height,
                        depth: 1,
                    },
                }],
            );
        }
    }

    /// Upload image to GPU (and block), making it available to shaders.
    pub fn upload_to_gpu_oneshot<C>(
        &self,
        command_pool: &mut CommandPool<B, C>,
        queue_group: &mut QueueGroup<B, C>,
    ) -> Result<(), failure::Error>
    where
        C: Capability + Supports<Transfer>,
    {
        // copy buffer to texture
        let mut copy_fence = self
            .device
            .create_fence(false)
            .with_context(|_| format_err!("Could not create fence"))?;

        unsafe {
            let mut cmd_buffer = command_pool.acquire_command_buffer::<OneShot>();
            cmd_buffer.begin();

            cmd_buffer.pipeline_barrier(
                PipelineStage::TOP_OF_PIPE..PipelineStage::TRANSFER,
                Dependencies::empty(),
                &[self.as_barrier(
                    (Access::empty(), Layout::Undefined)
                        ..(Access::TRANSFER_WRITE, Layout::TransferDstOptimal),
                )],
            );

            self.copy_buffer_to_image(&mut cmd_buffer);

            cmd_buffer.pipeline_barrier(
                PipelineStage::TRANSFER..PipelineStage::FRAGMENT_SHADER,
                Dependencies::empty(),
                &[self.as_barrier(
                    (Access::TRANSFER_WRITE, Layout::TransferDstOptimal)
                        ..(Access::SHADER_READ, Layout::ShaderReadOnlyOptimal),
                )],
            );

            cmd_buffer.finish();

            queue_group.queues[0].submit_nosemaphores(Some(&cmd_buffer), Some(&mut copy_fence));

            self.device
                .wait_for_fence(&copy_fence, !0)
                .with_context(|_| format_err!("Can't wait for fence"))?;
        }

        unsafe {
            self.device.destroy_fence(copy_fence);
        }

        Ok(())
    }
}

impl<'a, D, B> Drop for Image<'a, D, B>
where
    D: hal::Device<B>,
    B: hal::Backend,
{
    fn drop(&mut self) {
        use std::mem;

        unsafe {
            self.device
                .destroy_image_view(mem::replace(&mut self.image_view, mem::zeroed()));
            self.device
                .destroy_image(mem::replace(&mut self.image, mem::zeroed()));
            self.device
                .destroy_buffer(mem::replace(&mut self.image_upload_buffer, mem::zeroed()));
            self.device
                .free_memory(mem::replace(&mut self.image_memory, mem::zeroed()));
            self.device
                .free_memory(mem::replace(&mut self.image_upload_memory, mem::zeroed()));
        }
    }
}
