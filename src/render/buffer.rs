use super::texture::Texture;
use bytemuck::Pod;

pub struct Staged<B> {
    buffer: B,
}

impl Staged<wgpu::Buffer> {
    pub fn with_size(
        device: &wgpu::Device,
        size: wgpu::BufferAddress,
        usage: wgpu::BufferUsage,
    ) -> Self {
        Self {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size,
                usage: usage | wgpu::BufferUsage::COPY_DST,
            }),
        }
    }

    pub fn with_data<A: Pod>(device: &wgpu::Device, data: &[A], usage: wgpu::BufferUsage) -> Self {
        Self {
            buffer: device.create_buffer_with_data(
                bytemuck::cast_slice(data),
                usage | wgpu::BufferUsage::COPY_DST,
            ),
        }
    }

    pub fn update<A: Pod>(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        data: &[A],
    ) {
        let raw = bytemuck::cast_slice(data);
        let staging_buffer = device.create_buffer_with_data(raw, wgpu::BufferUsage::COPY_SRC);
        encoder.copy_buffer_to_buffer(
            &staging_buffer,
            0,
            &self.buffer,
            0,
            raw.len() as wgpu::BufferAddress,
        );
    }
}

impl Staged<Texture> {
    pub fn new(buffer: Texture) -> Self {
        Self { buffer }
    }

    pub fn update<A: Pod>(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        data: &[A],
        (width, height): (u32, u32),
    ) {
        let raw = bytemuck::cast_slice(data);
        let old_size = self.buffer.size;
        if old_size.width != width || old_size.height != height {
            self.buffer = Texture::create_ir_texture(device, (width, height));
        }
        let staging_buffer = device.create_buffer_with_data(raw, wgpu::BufferUsage::COPY_SRC);
        encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer: &staging_buffer,
                offset: 0,
                bytes_per_row: width * std::mem::size_of::<A>() as u32,
                rows_per_image: height,
            },
            wgpu::TextureCopyView {
                texture: &self.buffer.texture,
                mip_level: 0,
                array_layer: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            self.buffer.size,
        );
    }
}

impl<B> std::ops::Deref for Staged<B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
