#![cfg_attr(
    not(any(
        feature = "vulkan",
        feature = "dx12",
        feature = "metal",
        feature = "gl"
    )),
    allow(dead_code, unused_extern_crates, unused_imports)
)]

#[cfg(feature = "dx12")]
extern crate gfx_backend_dx12 as back;
#[cfg(feature = "gl")]
extern crate gfx_backend_gl as back;
#[cfg(feature = "metal")]
extern crate gfx_backend_metal as back;
#[cfg(feature = "vulkan")]
extern crate gfx_backend_vulkan as back;

use failure::{format_err, ResultExt};
use hal::format::{ChannelType, Swizzle};
use hal::pso::{PipelineStage, ShaderStageFlags};
use hal::queue::Submission;
use hal::{command, format as f, image as i, pass, pool, pso, window::Extent2D};
use hal::{Backbuffer, DescriptorPool, FrameSync, SwapchainConfig};
use hal::{Device, Instance, PhysicalDevice, Surface, Swapchain};
use setmod_gfx::{GraphicsPipeline, Image, Shader, ShaderSet, Vertex2d, VertexBuffer};
use std::path::Path;

#[cfg_attr(rustfmt, rustfmt_skip)]
const DIMS: Extent2D = Extent2D { width: 1920, height: 1080 };

#[cfg_attr(rustfmt, rustfmt_skip)]
const QUAD: [Vertex2d; 6] = [
    Vertex2d { pos: [ -0.5, 0.33 ], uv: [0.0, 1.0] },
    Vertex2d { pos: [  0.5, 0.33 ], uv: [1.0, 1.0] },
    Vertex2d { pos: [  0.5,-0.33 ], uv: [1.0, 0.0] },

    Vertex2d { pos: [ -0.5, 0.33 ], uv: [0.0, 1.0] },
    Vertex2d { pos: [  0.5,-0.33 ], uv: [1.0, 0.0] },
    Vertex2d { pos: [ -0.5,-0.33 ], uv: [0.0, 0.0] },
];

const COLOR_RANGE: i::SubresourceRange = i::SubresourceRange {
    aspects: f::Aspects::COLOR,
    levels: 0..1,
    layers: 0..1,
};

#[cfg(any(
    feature = "vulkan",
    feature = "dx12",
    feature = "metal",
    feature = "gl"
))]
fn main() -> Result<(), failure::Error> {
    pretty_env_logger::init();

    let mut events_loop = winit::EventsLoop::new();

    let wb = winit::WindowBuilder::new()
        .with_dimensions(winit::dpi::LogicalSize::new(
            DIMS.width as _,
            DIMS.height as _,
        ))
        .with_title("quad".to_string());
    // instantiate backend
    #[cfg(not(feature = "gl"))]
    let (_window, _instance, mut adapters, mut surface) = {
        let window = wb.build(&events_loop)?;
        let instance = back::Instance::create("gfx-rs quad", 1);
        let surface = instance.create_surface(&window);
        let adapters = instance.enumerate_adapters();
        (window, instance, adapters, surface)
    };
    #[cfg(feature = "gl")]
    let (mut adapters, mut surface) = {
        let window = {
            let builder =
                back::config_context(back::glutin::ContextBuilder::new(), ColorFormat::SELF, None)
                    .with_vsync(true);
            back::glutin::GlWindow::new(wb, builder, &events_loop)?
        };

        let surface = back::Surface::from_window(window);
        let adapters = surface.enumerate_adapters();
        (adapters, surface)
    };

    for adapter in &adapters {
        println!("{:?}", adapter.info);
    }

    let mut adapter = adapters.remove(0);
    let memory = adapter.physical_device.memory_properties();
    let limits = adapter.physical_device.limits();

    // Build a new device and associated command queues
    let (device, mut queue_group) =
        adapter.open_with::<_, hal::Graphics>(1, |family| surface.supports_queue_family(family))?;

    let mut command_pool = unsafe {
        device.create_command_pool_typed(&queue_group, pool::CommandPoolCreateFlags::empty())
    }
    .with_context(|_| format_err!("Can't create command pool"))?;

    // Setup renderpass and pipeline
    let set_layout = unsafe {
        device.create_descriptor_set_layout(
            &[
                pso::DescriptorSetLayoutBinding {
                    binding: 0,
                    ty: pso::DescriptorType::SampledImage,
                    count: 1,
                    stage_flags: ShaderStageFlags::FRAGMENT,
                    immutable_samplers: false,
                },
                pso::DescriptorSetLayoutBinding {
                    binding: 1,
                    ty: pso::DescriptorType::Sampler,
                    count: 1,
                    stage_flags: ShaderStageFlags::FRAGMENT,
                    immutable_samplers: false,
                },
            ],
            &[],
        )
    }
    .with_context(|_| format_err!("Can't create descriptor set layout"))?;

    // Descriptors
    let mut desc_pool = unsafe {
        device.create_descriptor_pool(
            1, // sets
            &[
                pso::DescriptorRangeDesc {
                    ty: pso::DescriptorType::SampledImage,
                    count: 1,
                },
                pso::DescriptorRangeDesc {
                    ty: pso::DescriptorType::Sampler,
                    count: 1,
                },
            ],
            pso::DescriptorPoolCreateFlags::empty(),
        )
    }
    .with_context(|_| format_err!("Can't create descriptor pool"))?;
    let desc_set = unsafe { desc_pool.allocate_set(&set_layout) }?;

    // Buffer allocations
    println!("Memory types: {:?}", memory.memory_types);

    let vertex_buffer = VertexBuffer::load_2d(&device, &QUAD, &memory)?;

    let image = Image::load_png(&device, limits, &memory, Path::new("src/data/logo.png"))?;

    let sampler = unsafe {
        device.create_sampler(i::SamplerInfo::new(i::Filter::Linear, i::WrapMode::Clamp))
    }
    .with_context(|_| format_err!("Can't create sampler"))?;

    unsafe {
        device.write_descriptor_sets(vec![
            pso::DescriptorSetWrite {
                set: &desc_set,
                binding: 0,
                array_offset: 0,
                descriptors: Some(image.as_descriptor()),
            },
            pso::DescriptorSetWrite {
                set: &desc_set,
                binding: 1,
                array_offset: 0,
                descriptors: Some(pso::Descriptor::Sampler(&sampler)),
            },
        ]);
    }

    image.upload_to_gpu_oneshot(&mut command_pool, &mut queue_group)?;

    let (caps, formats, _present_modes) = surface.compatibility(&mut adapter.physical_device);
    println!("formats: {:?}", formats);
    let format = formats.map_or(f::Format::Rgba8Srgb, |formats| {
        formats
            .iter()
            .find(|format| format.base_format().1 == ChannelType::Srgb)
            .map(|format| *format)
            .unwrap_or(formats[0])
    });

    let swap_config = SwapchainConfig::from_caps(&caps, format, DIMS);
    println!("{:?}", swap_config);
    let extent = swap_config.extent.to_extent();

    let (mut swap_chain, mut backbuffer) =
        unsafe { device.create_swapchain(&mut surface, swap_config, None) }
            .with_context(|_| format_err!("Can't create swapchain"))?;

    let render_pass = {
        let attachment = pass::Attachment {
            format: Some(format),
            samples: 1,
            ops: pass::AttachmentOps::new(
                pass::AttachmentLoadOp::Clear,
                pass::AttachmentStoreOp::Store,
            ),
            stencil_ops: pass::AttachmentOps::DONT_CARE,
            layouts: i::Layout::Undefined..i::Layout::Present,
        };

        let subpass = pass::SubpassDesc {
            colors: &[(0, i::Layout::ColorAttachmentOptimal)],
            depth_stencil: None,
            inputs: &[],
            resolves: &[],
            preserves: &[],
        };

        let dependency = pass::SubpassDependency {
            passes: pass::SubpassRef::External..pass::SubpassRef::Pass(0),
            stages: PipelineStage::COLOR_ATTACHMENT_OUTPUT..PipelineStage::COLOR_ATTACHMENT_OUTPUT,
            accesses: i::Access::empty()
                ..(i::Access::COLOR_ATTACHMENT_READ | i::Access::COLOR_ATTACHMENT_WRITE),
        };

        unsafe { device.create_render_pass(&[attachment], &[subpass], &[dependency]) }
            .with_context(|_| format_err!("Can't create render pass"))?
    };
    let (mut frame_images, mut framebuffers) = match backbuffer {
        Backbuffer::Images(images) => {
            let pairs = images
                .into_iter()
                .map(|image| unsafe {
                    let res = device.create_image_view(
                        &image,
                        i::ViewKind::D2,
                        format,
                        Swizzle::NO,
                        COLOR_RANGE.clone(),
                    );

                    res.map(move |rtv| (image, rtv))
                })
                .collect::<Result<Vec<_>, _>>()?;

            let fbos = pairs
                .iter()
                .map(|&(_, ref rtv)| unsafe {
                    device.create_framebuffer(&render_pass, Some(rtv), extent)
                })
                .collect::<Result<Vec<_>, _>>()?;

            (pairs, fbos)
        }
        Backbuffer::Framebuffer(fbo) => (Vec::new(), vec![fbo]),
    };

    // Define maximum number of frames we want to be able to be "in flight" (being computed
    // simultaneously) at once
    let frames_in_flight = 3;

    // Number of image acquisition semaphores is based on the number of swapchain images, not frames in flight,
    // plus one extra which we can guarantee is unused at any given time by swapping it out with the ones
    // in the rest of the queue.
    let mut image_acquire_semaphores = Vec::with_capacity(frame_images.len());
    let mut free_acquire_semaphore = device
        .create_semaphore()
        .with_context(|_| format_err!("Could not create semaphore"))?;

    // The number of the rest of the resources is based on the frames in flight.
    let mut submission_complete_semaphores = Vec::with_capacity(frames_in_flight);
    let mut submission_complete_fences = Vec::with_capacity(frames_in_flight);
    // Note: We don't really need a different command pool per frame in such a simple demo like this,
    // but in a more 'real' application, it's generally seen as optimal to have one command pool per
    // thread per frame. There is a flag that lets a command pool reset individual command buffers
    // which are created from it, but by default the whole pool (and therefore all buffers in it)
    // must be reset at once. Furthermore, it is often the case that resetting a whole pool is actually
    // faster and more efficient for the hardware than resetting individual command buffers, so it's
    // usually best to just make a command pool for each set of buffers which need to be reset at the
    // same time (each frame). In our case, each pool will only have one command buffer created from it,
    // though.
    let mut cmd_pools = Vec::with_capacity(frames_in_flight);
    let mut cmd_buffers = Vec::with_capacity(frames_in_flight);

    cmd_pools.push(command_pool);
    for _ in 1..frames_in_flight {
        unsafe {
            cmd_pools.push(
                device
                    .create_command_pool_typed(&queue_group, pool::CommandPoolCreateFlags::empty())
                    .with_context(|_| format_err!("Can't create command pool"))?,
            );
        }
    }

    for i in 0..frames_in_flight {
        image_acquire_semaphores.push(
            device
                .create_semaphore()
                .with_context(|_| format_err!("Could not create semaphore"))?,
        );
        submission_complete_semaphores.push(
            device
                .create_semaphore()
                .with_context(|_| format_err!("Could not create semaphore"))?,
        );
        submission_complete_fences.push(
            device
                .create_fence(true)
                .with_context(|_| format_err!("Could not create semaphore"))?,
        );
        cmd_buffers.push(cmd_pools[i].acquire_command_buffer::<command::MultiShot>());
    }

    let pipeline_layout = unsafe {
        device.create_pipeline_layout(
            std::iter::once(&set_layout),
            &[(pso::ShaderStageFlags::VERTEX, 0..8)],
        )
    }
    .with_context(|_| format_err!("Can't create pipeline layout"))?;

    let shader_set = ShaderSet::new(Shader::open(
        &device,
        "src/data/quad.vert",
        glsl_to_spirv::ShaderType::Vertex,
    )?)
    .fragment_shader(Shader::open(
        &device,
        "src/data/quad.frag",
        glsl_to_spirv::ShaderType::Fragment,
    )?);

    let pipeline = GraphicsPipeline::new(&device, &render_pass, &pipeline_layout, shader_set)?;

    // Rendering setup
    let mut viewport = pso::Viewport {
        rect: pso::Rect {
            x: 0,
            y: 0,
            w: extent.width as _,
            h: extent.height as _,
        },
        depth: 0.0..1.0,
    };

    //
    let mut running = true;
    let mut recreate_swapchain = false;
    let mut resize_dims = Extent2D {
        width: 0,
        height: 0,
    };
    let mut frame: u64 = 0;
    while running {
        events_loop.poll_events(|event| {
            if let winit::Event::WindowEvent { event, .. } = event {
                #[allow(unused_variables)]
                match event {
                    winit::WindowEvent::KeyboardInput {
                        input:
                            winit::KeyboardInput {
                                virtual_keycode: Some(winit::VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    }
                    | winit::WindowEvent::CloseRequested => running = false,
                    winit::WindowEvent::Resized(dims) => {
                        println!("resized to {:?}", dims);
                        #[cfg(feature = "gl")]
                        surface
                            .get_window()
                            .resize(dims.to_physical(surface.get_window().get_hidpi_factor()));
                        recreate_swapchain = true;
                        resize_dims.width = dims.width as u32;
                        resize_dims.height = dims.height as u32;
                    }
                    _ => (),
                }
            }
        });

        // Window was resized so we must recreate swapchain and framebuffers
        if recreate_swapchain {
            device.wait_idle()?;

            let (caps, formats, _present_modes) =
                surface.compatibility(&mut adapter.physical_device);
            // Verify that previous format still exists so we may reuse it.
            assert!(formats.iter().any(|fs| fs.contains(&format)));

            let swap_config = SwapchainConfig::from_caps(&caps, format, resize_dims);
            println!("{:?}", swap_config);
            let extent = swap_config.extent.to_extent();

            let (new_swap_chain, new_backbuffer) =
                unsafe { device.create_swapchain(&mut surface, swap_config, Some(swap_chain)) }
                    .with_context(|_| format_err!("Can't create swapchain"))?;

            unsafe {
                // Clean up the old framebuffers, images and swapchain
                for framebuffer in framebuffers {
                    device.destroy_framebuffer(framebuffer);
                }
                for (_, rtv) in frame_images {
                    device.destroy_image_view(rtv);
                }
            }

            backbuffer = new_backbuffer;
            swap_chain = new_swap_chain;

            let (new_frame_images, new_framebuffers) = match backbuffer {
                Backbuffer::Images(images) => {
                    let pairs = images
                        .into_iter()
                        .map(|image| unsafe {
                            let res = device.create_image_view(
                                &image,
                                i::ViewKind::D2,
                                format,
                                Swizzle::NO,
                                COLOR_RANGE.clone(),
                            );

                            res.map(move |rtv| (image, rtv))
                        })
                        .collect::<Result<Vec<_>, _>>()?;

                    let fbos = pairs
                        .iter()
                        .map(|&(_, ref rtv)| unsafe {
                            device.create_framebuffer(&render_pass, Some(rtv), extent)
                        })
                        .collect::<Result<Vec<_>, _>>()?;

                    (pairs, fbos)
                }
                Backbuffer::Framebuffer(fbo) => (Vec::new(), vec![fbo]),
            };

            framebuffers = new_framebuffers;
            frame_images = new_frame_images;
            viewport.rect.w = extent.width as _;
            viewport.rect.h = extent.height as _;
            recreate_swapchain = false;
        }

        // Use guaranteed unused acquire semaphore to get the index of the next frame we will render to
        // by using acquire_image
        let swap_image = unsafe {
            match swap_chain.acquire_image(
                u64::max_value(),
                FrameSync::Semaphore(&free_acquire_semaphore),
            ) {
                Ok(i) => i as usize,
                Err(_) => {
                    recreate_swapchain = true;
                    continue;
                }
            }
        };

        // Swap the acquire semaphore with the one previously associated with the image we are acquiring
        std::mem::swap(
            &mut free_acquire_semaphore,
            &mut image_acquire_semaphores[swap_image],
        );

        // Compute index into our resource ring buffers based on the frame number
        // and number of frames in flight. Pay close attention to where this index is needed
        // versus when the swapchain image index we got from acquire_image is needed.
        let frame_idx = frame as usize % frames_in_flight;

        // Wait for the fence of the previous submission of this frame and reset it; ensures we are
        // submitting only up to maximum number of frames_in_flight if we are submitting faster than
        // the gpu can keep up with. This would also guarantee that any resources which need to be
        // updated with a CPU->GPU data copy are not in use by the GPU, so we can perform those updates.
        // In this case there are none to be done, however.
        unsafe {
            device
                .wait_for_fence(&submission_complete_fences[frame_idx], !0)
                .with_context(|_| format_err!("Failed to wait for fence"))?;
            device
                .reset_fence(&submission_complete_fences[frame_idx])
                .with_context(|_| format_err!("Failed to reset fence"))?;
            cmd_pools[frame_idx].reset();
        }

        // Rendering
        let cmd_buffer = &mut cmd_buffers[frame_idx];
        unsafe {
            cmd_buffer.begin(false);

            cmd_buffer.set_viewports(0, &[viewport.clone()]);
            cmd_buffer.set_scissors(0, &[viewport.rect]);
            pipeline.bind(cmd_buffer);
            vertex_buffer.bind(cmd_buffer, 0, 0);
            cmd_buffer.bind_graphics_descriptor_sets(&pipeline_layout, 0, Some(&desc_set), &[]);

            {
                let mut encoder = cmd_buffer.begin_render_pass_inline(
                    &render_pass,
                    &framebuffers[swap_image],
                    viewport.rect,
                    &[command::ClearValue::Color(command::ClearColor::Float([
                        0.0, 0.0, 0.0, 1.0,
                    ]))],
                );
                encoder.draw(0..6, 0..1);
            }

            cmd_buffer.finish();

            let submission = Submission {
                command_buffers: Some(&*cmd_buffer),
                wait_semaphores: Some((
                    &image_acquire_semaphores[swap_image],
                    PipelineStage::COLOR_ATTACHMENT_OUTPUT,
                )),
                signal_semaphores: Some(&submission_complete_semaphores[frame_idx]),
            };
            queue_group.queues[0].submit(submission, Some(&submission_complete_fences[frame_idx]));

            // present frame
            if let Err(_) = swap_chain.present(
                &mut queue_group.queues[0],
                swap_image as hal::SwapImageIndex,
                Some(&submission_complete_semaphores[frame_idx]),
            ) {
                recreate_swapchain = true;
            }
        }
        // Increment our frame
        frame += 1;
    }

    // cleanup!
    device.wait_idle()?;
    unsafe {
        device.destroy_descriptor_pool(desc_pool);
        device.destroy_descriptor_set_layout(set_layout);

        device.destroy_sampler(sampler);
        device.destroy_semaphore(free_acquire_semaphore);
        for p in cmd_pools {
            device.destroy_command_pool(p.into_raw());
        }
        for s in image_acquire_semaphores {
            device.destroy_semaphore(s);
        }
        for s in submission_complete_semaphores {
            device.destroy_semaphore(s);
        }
        for f in submission_complete_fences {
            device.destroy_fence(f);
        }
        device.destroy_render_pass(render_pass);
        device.destroy_pipeline_layout(pipeline_layout);
        for framebuffer in framebuffers {
            device.destroy_framebuffer(framebuffer);
        }
        for (_, rtv) in frame_images {
            device.destroy_image_view(rtv);
        }

        device.destroy_swapchain(swap_chain);
    }

    Ok(())
}

#[cfg(not(any(
    feature = "vulkan",
    feature = "dx12",
    feature = "metal",
    feature = "gl"
)))]
fn main() {
    println!("You need to enable the native API feature (vulkan/metal) in order to test the LL");
}
