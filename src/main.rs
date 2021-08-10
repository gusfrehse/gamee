use vulkano::buffer::cpu_access::CpuAccessibleBuffer;
use vulkano::buffer::BufferUsage;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryCommandBuffer};
use vulkano::device::physical::PhysicalDevice;
use vulkano::device::{Device, DeviceExtensions, Features};
use vulkano::instance::{Instance, InstanceExtensions};
use vulkano::sync::GpuFuture;
use vulkano::Version;

fn main() {
    let instance = Instance::new(None, Version::V1_1, &InstanceExtensions::none(), None)
        .expect("failed to create instance");
    let physical = PhysicalDevice::enumerate(&instance)
        .next()
        .expect("no device avaiable");

    for family in physical.queue_families() {
        println!(
            "queue family with {:?} queues. Graphics? {:?}",
            family.queues_count(),
            family.supports_graphics()
        );
    }

    let queue_family = physical
        .queue_families()
        .find(|&q| q.supports_graphics())
        .expect("couldn't find a graphical queue family");

    let (device, mut queues) = {
        Device::new(
            physical,
            &Features::none(),
            &DeviceExtensions::none(),
            [(queue_family, 0.5)].iter().cloned(),
        )
        .expect("failed to create device")
    };

    let queue = queues.next().unwrap();

    let source_content = 0..64;
    let source =
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, source_content)
            .expect("failed to create buffer");

    let dest_content = (0..64).map(|_| 0);
    let dest =
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, dest_content)
            .expect("failed to create buffer");

    let mut builder = AutoCommandBufferBuilder::primary(
        device.clone(),
        queue.family(),
        CommandBufferUsage::MultipleSubmit,
    )
    .unwrap();
    builder.copy_buffer(source.clone(), dest.clone()).unwrap();
    let command_buffer = builder.build().unwrap();
    let finished = command_buffer.execute(queue.clone()).unwrap();
    finished
        .then_signal_fence_and_flush()
        .unwrap()
        .wait(None)
        .unwrap();

    let src_content = source.read().unwrap();
    let dest_content = dest.read().unwrap();
    assert_eq!(&*src_content, &*dest_content);
}
