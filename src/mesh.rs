use anyhow::*;

pub struct MeshDescriptor {
    vertices: Vec<cgmath::Point3<f32>>,
    normals: Vec<cgmath::Vector3<f32>>,
    uvs: Vec<cgmath::Point2<f32>>,
    triangles: Vec<u32>,
}

pub struct Mesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    textures: wgpu::BindGroup,
    uniforms: wgpu::BindGroup,
}
impl Mesh {
    fn draw(surface: &wgpu::Surface, device: &wgpu::Device) -> Result<()> {
        let frame = surface.get_current_frame()?.output;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Mesh Encoder"),
        });

        {
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Mesh Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);

            // Textures
            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);

            // Uniforms
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);

            // Vertices
            render_pass.set_vertex_buffer(0, self.vertices.slice(..));

            // Indices
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }
}
