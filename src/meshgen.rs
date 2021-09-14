use crate::mesh;
use crate::texture;
use image::{GenericImageView, Pixel};
use std::vec::Vec;

impl mesh::Descriptor {
    pub fn from_height_map(
        map: &image::DynamicImage,
        columns: i32,
        rows: i32,
        scale: f32,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Self {
        let (width, height) = (*map).dimensions();

        let mut vertices: Vec<[f32; 3]> = Vec::new();
        let mut normals: Vec<[f32; 3]> = Vec::new();
        let mut uvs: Vec<[f32; 2]> = Vec::new();
        let mut triangles: Vec<u32> = Vec::new();
        let mut texture: texture::Texture =
            texture::Texture::from_image(device, queue, &map, Some("Height Map Texture")).unwrap();

        let column_width = width as f32 / columns as f32;
        let row_height = height as f32 / rows as f32;

        for x in 0..columns {
            for y in 0..rows {
                let xpos = column_width * x as f32;
                let ypos = row_height * y as f32;

                let pixel = map.get_pixel(xpos as u32, ypos as u32);
                if let [r, g, b, a] = pixel.channels() {
                    vertices.push([xpos * scale, *r as f32 * scale, ypos * scale]);
                }

                normals.push(cgmath::Vector3::unit_y().into());

                uvs.push([xpos / width as f32, ypos / height as f32]);
            }
        }

        for x in 0..(columns - 1) {
            for y in 0..(rows - 1) {
                // first  triangle: [      y + x * rows, (y + 1) + x * rows,       y + (x + 1) * rows]
                triangles.push((y + x * rows) as u32);
                triangles.push((y + 1 + x * rows) as u32);
                triangles.push((y + (x + 1) * rows) as u32);

                // second triangle: [(y + 1) + x * rows, (y + 1) + (x + 1) * rows, y + (x + 1) * rows]
                triangles.push((y + 1 + x * rows) as u32);
                triangles.push((y + 1 + (x + 1) * rows) as u32);
                triangles.push((y + (x + 1) * rows) as u32);
            }
        }

        Self {
            vertices,
            normals,
            uvs,
            triangles,
            texture,
        }
    }
}
