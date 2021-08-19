use std::sync::Arc;
use vulkano::buffer::cpu_access::CpuAccessibleBuffer;
use vulkano::buffer::BufferUsage;
use vulkano::command_buffer::SubpassContents;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryCommandBuffer};
use vulkano::descriptor_set::persistent::PersistentDescriptorSet;
use vulkano::device::physical::PhysicalDevice;
use vulkano::device::{Device, DeviceExtensions, Features};
use vulkano::format::ClearValue;
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{ImageDimensions, StorageImage};
use vulkano::instance::{Instance, InstanceExtensions};
use vulkano::pipeline::ComputePipeline;
use vulkano::pipeline::ComputePipelineAbstract;
use vulkano::render_pass::Framebuffer;
use vulkano::sync::GpuFuture;
use vulkano::Version;

use image::{ImageBuffer, Rgba};

mod cs {
    vulkano_shaders::shader! {
    ty: "compute",
    path: "src/mandelbrot.glsl"
    }
}

#[derive(Default, Copy, Clone)]
struct Vertex {
    pos: [f32; 2],
}

vulkano::impl_vertex!(Vertex, pos);

fn main() {
    // Instantiate vulkan
    let instance = Instance::new(None, Version::V1_1, &InstanceExtensions::none(), None)
        .expect("failed to create instance");

    // Use first physical device
    let physical = PhysicalDevice::enumerate(&instance)
        .next()
        .expect("no device avaiable");

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
        Device::new(
            physical,
            &Features::none(),
            &DeviceExtensions::none(),
            [(queue_family, 0.5)].iter().cloned(),
        )
        .expect("failed to create device")
    };

    // Get the first queue
    let queue = queues.next().unwrap();

    // Image in the device
    let image = StorageImage::new(
        device.clone(),
        ImageDimensions::Dim2d {
            width: 1024,
            height: 1024,
            array_layers: 1,
        },
        Format::R8G8B8A8Unorm,
        Some(queue.family()),
    )
    .unwrap();

    let view = ImageView::new(image.clone()).unwrap();

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

    let render_pass = Arc::new(
        vulkano::single_pass_renderpass!(device.clone(),
                     attachments: {
                         color: {
                             load: Clear,
                             store: Store,
                             format: Format::R8G8B8A8Unorm,
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

    let frambuffer = Arc::new(
        Framebuffer::start(render_pass.clone())
            .add(view)
            .unwrap()
            .build()
            .unwrap(),
    );

    let shader = cs::Shader::load(device.clone()).expect("failed to create shader module");

    let compute_pipeline = Arc::new(
        ComputePipeline::new(device.clone(), &shader.main_entry_point(), &(), None)
            .expect("failed to create compute pipeline"),
    );

    let layout = compute_pipeline
        .layout()
        .descriptor_set_layouts()
        .get(0)
        .unwrap();

    let set = Arc::new(
        PersistentDescriptorSet::start(layout.clone())
            .add_image(view.clone())
            .unwrap()
            .build()
            .unwrap(),
    );

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
        .end_render_pass()
        .unwrap();

    let command_buffer = builder.build().unwrap();

    let finished = command_buffer.execute(queue.clone()).unwrap();

    finished
        .then_signal_fence_and_flush()
        .unwrap()
        .wait(None)
        .unwrap();

    let buffer_content = buf.read().unwrap();
    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(4096, 4096, &buffer_content[..]).unwrap();
    image.save("out.png").unwrap();

    println!("\nEverything succeded!");
}
