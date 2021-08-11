use std::sync::Arc;
use vulkano::buffer::cpu_access::CpuAccessibleBuffer;
use vulkano::buffer::BufferUsage;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryCommandBuffer};
use vulkano::descriptor_set::persistent::PersistentDescriptorSet;
use vulkano::device::physical::PhysicalDevice;
use vulkano::device::{Device, DeviceExtensions, Features};
use vulkano::instance::{Instance, InstanceExtensions};
use vulkano::pipeline::layout::PipelineLayout;
use vulkano::pipeline::ComputePipeline;
use vulkano::pipeline::ComputePipelineAbstract;
use vulkano::sync::GpuFuture;
use vulkano::Version;

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

    // Create source buffer
    let source_content = 0..64;
    let source =
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, source_content)
            .expect("failed to create buffer");

    // Create a destination buffer
    let dest_content = (0..64).map(|_| 0);
    let dest =
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, dest_content)
            .expect("failed to create buffer");

    // Create a builder that will batch the commands
    let mut builder = AutoCommandBufferBuilder::primary(
        device.clone(),
        queue.family(),
        CommandBufferUsage::MultipleSubmit,
    )
    .unwrap();

    // Send the command copy buffer to the builder
    builder.copy_buffer(source.clone(), dest.clone()).unwrap();

    // Build the batch commands
    let command_buffer = builder.build().unwrap();

    // Execute the commands and wait for them to finish
    let finished = command_buffer.execute(queue.clone()).unwrap();
    finished
        .then_signal_fence_and_flush()
        .unwrap()
        .wait(None)
        .unwrap();

    // Read both buffers and assert they are equal
    let src_content = source.read().unwrap();
    let dest_content = dest.read().unwrap();
    assert_eq!(&*src_content, &*dest_content);

    let data_iter = 0..65536;
    let data_buffer =
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, data_iter)
            .expect("failed to build buffer");

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
            .add_buffer(data_buffer.clone())
            .unwrap()
            .build()
            .unwrap(),
    );

    let mut builder = AutoCommandBufferBuilder::primary(
        device.clone(),
        queue.family(),
        CommandBufferUsage::MultipleSubmit,
    )
    .unwrap();

    builder
        .dispatch([1024, 1, 1], compute_pipeline.clone(), set.clone(), ())
        .unwrap();
    let command_buffer = builder.build().unwrap();

    let finished = command_buffer.execute(queue.clone()).unwrap();
    finished
        .then_signal_fence_and_flush()
        .unwrap()
        .wait(None)
        .unwrap();
    let content = data_buffer.read().unwrap();
    for (n, val) in content.iter().enumerate() {
        if n % 1000 == 0 {
            println!("{} {}", n, *val);
        }
    }
    println!("\nEverything succeded!");
}

mod cs {
    vulkano_shaders::shader! {
    ty: "compute",
    src: "
#version 450

layout(local_size_x = 64, local_size_y = 1, local_size_z = 1) in;

layout(set = 0, binding = 0) buffer Data {
    uint data[];
} buf;

void main() {
    uint idx = gl_GlobalInvocationID.x;
    buf.data[idx] *= 12;
}"
    }
}
