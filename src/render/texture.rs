use iced_wgpu::wgpu;

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub size: wgpu::Extent3d,
}

impl Texture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create_normal_texture(device: &wgpu::Device, size: (u32, u32)) -> Self {
        let size = wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Normal Texture"),
            size,
            mip_level_count: 1,
            array_layer_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsage::STORAGE | wgpu::TextureUsage::SAMPLED,
        });
        let view = texture.create_default_view();
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare: wgpu::CompareFunction::Undefined,
        });

        Self {
            size,
            texture,
            view,
            sampler,
        }
    }

    pub fn create_ir_texture(device: &wgpu::Device, size: (u32, u32)) -> Self {
        let size = wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("IR Texture"),
            size,
            mip_level_count: 1,
            array_layer_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::SAMPLED,
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_default_view();
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare: wgpu::CompareFunction::Undefined,
        });

        Self {
            texture,
            view,
            sampler,
            size,
        }
    }

    pub fn create_depth_texture(
        device: &wgpu::Device,
        sc_desc: &wgpu::SwapChainDescriptor,
        sample_count: u32,
        label: Option<&str>,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: sc_desc.width,
            height: sc_desc.height,
            depth: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            array_layer_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT
                | wgpu::TextureUsage::SAMPLED
                | wgpu::TextureUsage::COPY_SRC,
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_default_view();
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare: wgpu::CompareFunction::LessEqual,
        });

        Self {
            texture,
            view,
            sampler,
            size,
        }
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        texture: image::GrayImage,
    ) {
        let flat_samples = texture.as_flat_samples();
        let old_size = self.size;
        let (width, height) = texture.dimensions();
        if old_size.width != width || old_size.height != height {
            *self = Texture::create_ir_texture(device, (width, height));
        }
        let staging_buffer =
            device.create_buffer_with_data(flat_samples.samples, wgpu::BufferUsage::COPY_SRC);
        encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer: &staging_buffer,
                offset: 0,
                bytes_per_row: flat_samples.layout.height_stride as u32,
                rows_per_image: 0,
            },
            wgpu::TextureCopyView {
                texture: &self.texture,
                mip_level: 0,
                array_layer: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            self.size,
        );
    }
}
