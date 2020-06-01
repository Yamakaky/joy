use bytemuck::Pod;
use iced_wgpu::wgpu;

pub trait Staged {
    fn update<A: Pod>(&self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, data: &[A]);
}

impl Staged for wgpu::Buffer {
    fn update<A: Pod>(
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
            self,
            0,
            raw.len() as wgpu::BufferAddress,
        );
    }
}
