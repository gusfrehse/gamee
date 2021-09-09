use anyhow::*;
use wgpu::util::DeviceExt;

use crate::state;
use crate::texture;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

pub struct MeshDescriptor {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub triangles: Vec<u32>,
    pub texture: texture::Texture,
}

pub struct Mesh {
    pub vertices: wgpu::Buffer,
    pub indices: wgpu::Buffer,
    pub indices_count: u32,
    pub texture: wgpu::BindGroup,
    pub texture_layout: wgpu::BindGroupLayout,
}

impl MeshDescriptor {
    pub fn bake(&self, device: &wgpu::Device) -> Mesh {
        // TODO: Currently sending an array of structs, could send various arrays
        // (more than one buffer) as we already are storing in that format.
        // Need to change how vertex attributes are declared.
        // works for now though.

        // Really crazy vector manipulation.
        // Basically collapsing each of vertices, normals and uvs to a single
        // vertex and making a vector from them.
        let vertices_with_attributes = self
            .vertices
            .iter()
            .zip(self.normals.iter().zip(self.uvs.iter()))
            .map(|(v, (n, u))| Vertex {
                position: v.clone(),
                normal: n.clone(),
                uv: u.clone(),
            })
            .collect::<Vec<Vertex>>();

        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            usage: wgpu::BufferUsages::VERTEX,
            contents: bytemuck::cast_slice(&vertices_with_attributes[..]),
        });

        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            usage: wgpu::BufferUsages::INDEX,
            contents: bytemuck::cast_slice(&self.triangles[..]),
        });

        let indices_count = self.triangles.len() as u32;

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        comparison: false,
                        filtering: true,
                    },
                    count: None,
                },
            ],
            label: Some("Texture Bind Group Layout"),
        });

        let texture = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.texture.sampler),
                },
            ],
            label: Some("Texture Bind Group"),
        });

        Mesh {
            vertices,
            indices,
            indices_count,
            texture,
            texture_layout,
        }
    }
}

// TODO: Change to something useful (need to pass self as parameter at least!!).
// We are currently using the surface, the device and the camera bind group (uniform).
// So we would need to abstract that away for it to work...
impl Mesh {
    pub fn draw(state: &state::State) -> Result<()> {
        let frame = state.surface.get_current_frame()?.output;

        let mut encoder = state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
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
                        load: wgpu::LoadOp::Clear(state.clear_color),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&state.render_pipeline);

            // Textures
            render_pass.set_bind_group(0, &state.mesh.texture, &[]);

            // Uniforms
            render_pass.set_bind_group(1, &state.camera_bind_group, &[]);

            // Vertices
            render_pass.set_vertex_buffer(0, state.mesh.vertices.slice(..));

            // Indices
            render_pass.set_index_buffer(state.mesh.indices.slice(..), wgpu::IndexFormat::Uint32);

            render_pass.draw_indexed(0..state.mesh.indices_count, 0, 0..1);
        }

        state.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }
}
