use bytemuck::Pod;
use iced_wgpu::wgpu;
use std::{
    any::type_name,
    ops::{Deref, DerefMut},
};
use wgpu::util::DeviceExt;

pub trait Staged {
    fn update<A: Pod>(&self, queue: &mut wgpu::Queue, data: &[A]);
}

impl Staged for wgpu::Buffer {
    fn update<A: Pod>(&self, queue: &mut wgpu::Queue, data: &[A]) {
        queue.write_buffer(self, 0, bytemuck::cast_slice(data));
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
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            usage: usage | wgpu::BufferUsage::COPY_DST,
            contents: bytemuck::bytes_of(&inner),
            label: Some("2D Vertex Buffer"),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility,
                ty: wgpu::BindingType::UniformBuffer {
                    dynamic: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some(&format!("{} bind group layout", type_name::<T>())),
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(buffer.slice(..)),
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

    pub fn upload(&mut self, queue: &mut wgpu::Queue) {
        if self.dirty {
            self.buffer.update(queue, &[self.inner]);
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
