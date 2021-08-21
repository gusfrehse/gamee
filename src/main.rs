use std::{mem::swap, sync::Arc};
use vulkano::{
    buffer::{cpu_access::CpuAccessibleBuffer, BufferUsage},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, DynamicState, PrimaryCommandBuffer,
        SubpassContents,
    },
    device::{physical::PhysicalDevice, Device, DeviceExtensions, Features},
    format::Format,
    image::{view::ImageView, ImageDimensions, ImageUsage, StorageImage},
    instance::{Instance, InstanceExtensions},
    pipeline::{viewport::Viewport, GraphicsPipeline},
    render_pass::{Framebuffer, Subpass},
    swapchain,
    swapchain::{
        ColorSpace, FullscreenExclusive, PresentMode, SurfaceTransform, Swapchain, SwapchainBuilder,
    },
    sync::GpuFuture,
    Version,
};

use vulkano_win::VkSurfaceBuild;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use image::{ImageBuffer, Rgba};

#[derive(Default, Copy, Clone)]
struct Vertex {
    pos: [f32; 2],
}

vulkano::impl_vertex!(Vertex, pos);

fn main() {
    // Instantiate vulkan
    let instance = {
        let extensions = vulkano_win::required_extensions();
        Instance::new(None, Version::V1_2, &extensions, None).expect("failed to create instance")
    };

    // Use first physical device
    let physical = PhysicalDevice::enumerate(&instance)
        .next()
        .expect("no device avaiable");

    // Event loop
    let event_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .build_vk_surface(&event_loop, instance.clone())
        .unwrap();

    // Find all families
    for family in physical.queue_families() {
        println!(
            "queue family with {:?} queues. Graphics? {:?}",
            family.queues_count(),
            family.supports_graphics()
        );
    }

    // This is the family that supports graphics
    let queue_family = physical
        .queue_families()
        .find(|&q| q.supports_graphics())
        .expect("couldn't find a graphical queue family");

    // Create a virtual (?) device
    let (device, mut queues) = {
        let device_ext = vulkano::device::DeviceExtensions {
            khr_swapchain: true,
            ..vulkano::device::DeviceExtensions::none()
        };
        Device::new(
            physical,
            &Features::none(),
            &device_ext,
            [(queue_family, 0.5)].iter().cloned(),
        )
        .expect("failed to create device")
    };

    // Get the first queue
    let queue = queues.next().unwrap();

    let (swapchain, images) = {
        let caps = surface.capabilities(physical).unwrap();
        let dimensions = surface.window().inner_size().into();
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();
        let format = caps.supported_formats[0].0;

        Swapchain::start(device.clone(), surface.clone())
            .num_images(caps.min_image_count)
            .format(format)
            .dimensions(dimensions)
            .layers(1)
            .usage(ImageUsage::color_attachment())
            .sharing_mode(&queue)
            .transform(SurfaceTransform::Identity)
            .composite_alpha(alpha)
            .present_mode(PresentMode::Fifo)
            .fullscreen_exclusive(FullscreenExclusive::Default)
            .clipped(true)
            .color_space(ColorSpace::SrgbNonLinear)
            .build()
            .expect("failed to create swapchain")
    };

    let vertex1 = Vertex { pos: [-0.5, -0.5] };
    let vertex2 = Vertex { pos: [0.0, 0.5] };
    let vertex3 = Vertex { pos: [0.5, -0.25] };

    let vertex_buffer = CpuAccessibleBuffer::from_iter(
        device.clone(),
        BufferUsage::all(),
        false,
        vec![vertex1, vertex2, vertex3].into_iter(),
    )
    .expect("failed to create buffer");

    mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            path: "src/vertex.glsl"
        }
    }

    mod fs {
        vulkano_shaders::shader! {
            ty: "fragment",
            path: "src/frag.glsl"
        }
    }

    let vs = vs::Shader::load(device.clone()).expect("failed to vs create shader module");
    let fs = fs::Shader::load(device.clone()).expect("failed to fs create shader module");

    let render_pass = Arc::new(
        vulkano::single_pass_renderpass!(device.clone(),
                     attachments: {
                         color: {
                             load: Clear,
                             store: Store,
                             format: swapchain.format(),
                             samples: 1,
                         }
                     },
                     pass: {
                         color: [color],
                         depth_stencil: {}
                     }
        )
        .unwrap(),
    );

    let pipeline = Arc::new(
        GraphicsPipeline::start()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(vs.main_entry_point(), ())
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fs.main_entry_point(), ())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap(),
    );

    let dynamic_state = DynamicState {
        line_width: None,
        viewports: None,
        scissors: None,
        compare_mask: None,
        write_mask: None,
        reference: None,
    };

    let frambuffer = Arc::new(
        Framebuffer::start(render_pass.clone())
            .add(view)
            .unwrap()
            .build()
            .unwrap(),
    );

    // End of vulkan/window initialization

    let mut builder = AutoCommandBufferBuilder::primary(
        device.clone(),
        queue.family(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();

    builder
        .begin_render_pass(
            frambuffer.clone(),
            SubpassContents::Inline,
            vec![[0.0, 0.0, 1.0, 1.0].into()],
        )
        .unwrap()
        .draw(
            pipeline.clone(),
            &dynamic_state,
            vertex_buffer.clone(),
            (),
            (),
        )
        .unwrap()
        .end_render_pass()
        .unwrap()
        .copy_image_to_buffer(image.clone(), buf.clone())
        .unwrap();

    let command_buffer = builder.build().unwrap();

    let finished = command_buffer.execute(queue.clone()).unwrap();

    finished
        .then_signal_fence_and_flush()
        .unwrap()
        .wait(None)
        .unwrap();

    let buffer_content = buf.read().unwrap();
    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(1024, 1024, &buffer_content[..]).unwrap();
    image.save("out.png").unwrap();

    let (image_num, suboptimal, acquire_future) =
        swapchain::acquire_next_image(swapchain.clone(), None).unwrap();

    if suboptimal {
        recreate_swapchain = true;
    }

    event_loop.run(|event, _, control_flow| match event {
        winit::event::Event::WindowEvent {
            event: winit::event::WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }
        _ => (),
    });

    println!("\nEverything succeded!");
}
