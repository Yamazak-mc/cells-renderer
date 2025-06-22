/// RGBA framebuffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldImage {
    width: u32,
    height: u32,
    buf: Vec<u8>,
}

impl WorldImage {
    const CHANNELS: usize = 4;

    #[inline]
    pub fn new(width: u32, height: u32) -> Self {
        assert!(width > 0 && height > 0);

        Self {
            width,
            height,
            buf: vec![0; width as usize * height as usize * Self::CHANNELS],
        }
    }

    #[inline]
    pub fn filled(width: u32, height: u32, color: [u8; 4]) -> Self {
        let mut this = Self::new(width, height);
        for pixel in this.buf.chunks_exact_mut(4) {
            pixel.copy_from_slice(&color);
        }
        this
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[inline]
    pub fn buf(&self) -> &[u8] {
        &self.buf
    }

    #[inline]
    pub fn buf_mut(&mut self) -> &mut [u8] {
        &mut self.buf
    }

    #[inline]
    pub fn get(&self, x: u32, y: u32) -> Option<&[u8]> {
        self.calc_offset(x, y)
            .map(|i| &self.buf[i..i + Self::CHANNELS])
    }

    #[inline]
    pub fn get_mut(&mut self, x: u32, y: u32) -> Option<&mut [u8]> {
        self.calc_offset(x, y)
            .map(|i| &mut self.buf[i..i + Self::CHANNELS])
    }

    fn calc_offset(&self, x: u32, y: u32) -> Option<usize> {
        (x < self.width && y < self.height)
            .then(|| (x as usize + y as usize * self.width as usize) * 4)
    }

    pub(crate) fn create_texture(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: Option<&str>,
    ) -> anyhow::Result<(wgpu::Texture, wgpu::TextureView, wgpu::Sampler)> {
        let texture_size = self.texture_size();

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        self.update_wgpu_texture(&texture, queue);

        Ok((texture, view, sampler))
    }

    pub(crate) fn update_wgpu_texture(&self, texture: &wgpu::Texture, queue: &wgpu::Queue) {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.buf,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.width()),
                rows_per_image: Some(self.height()),
            },
            self.texture_size(),
        );
        queue.submit([]);
    }

    fn texture_size(&self) -> wgpu::Extent3d {
        wgpu::Extent3d {
            width: self.width(),
            height: self.height(),
            depth_or_array_layers: 1,
        }
    }
}
