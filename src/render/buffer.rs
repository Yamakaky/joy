use bytemuck::Pod;
use iced_wgpu::wgpu;
use std::{
    any::type_name,
    ops::{Deref, DerefMut},
};

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

pub struct BoundBuffer<T> {
    inner: T,
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
    dirty: bool,
}

impl<T: Default + Pod> BoundBuffer<T> {
    pub fn new(
        device: &wgpu::Device,
        usage: wgpu::BufferUsage,
        visibility: wgpu::ShaderStage,
    ) -> Self {
        let inner = T::default();
        let raw_inner = bytemuck::bytes_of(&inner);
        let buffer = device.create_buffer_with_data(raw_inner, usage | wgpu::BufferUsage::COPY_DST);
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility,
                ty: wgpu::BindingType::UniformBuffer { dynamic: false },
            }],
            label: Some(&format!("{} bind group layout", type_name::<T>())),
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            bindings: &[wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::Buffer {
                    buffer: &buffer,
                    range: 0..raw_inner.len() as wgpu::BufferAddress,
                },
            }],
            label: Some(&format!("{} bind group", type_name::<T>())),
        });
        Self {
            inner,
            buffer,
            bind_group,
            bind_group_layout,
            dirty: true,
        }
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn upload(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder) {
        if self.dirty {
            self.buffer.update(device, encoder, &[self.inner]);
            self.dirty = false;
        }
    }
}

impl<T> Deref for BoundBuffer<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for BoundBuffer<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.dirty = true;
        &mut self.inner
    }
}
