use bytemuck::Pod;

pub struct StagedBuffer {
    buffer: wgpu::Buffer,
}

impl StagedBuffer {
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

impl std::ops::Deref for StagedBuffer {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
