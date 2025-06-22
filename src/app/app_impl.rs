use crate::{AppConfigs, MouseEvent, World, WorldImage};
use anyhow::Context as _;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use wgpu::util::DeviceExt as _;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

#[derive(Debug)]
pub struct AppImpl<'window, W> {
    // Configs
    configs: AppConfigs,

    // World
    world: W,
    world_image: WorldImage,
    world_aspect: f32,

    // Window
    window: Arc<Window>,
    window_size: PhysicalSize<u32>,

    // Update cycle
    update_interval: Duration,
    last_update: Instant,

    // Cursor
    bounds: WorldTransform,
    cursor_translated: Option<(u32, u32)>,

    // Pause
    paused: bool,

    // wgpu
    surface: wgpu::Surface<'window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,

    // Texture
    should_update_texture: bool,
    texture: wgpu::Texture,
    #[allow(unused)]
    texture_view: wgpu::TextureView,
    #[allow(unused)]
    texture_sampler: wgpu::Sampler,
    texture_bind_group: wgpu::BindGroup,

    // Rendering
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    indices_len: u32,
    render_pipeline: wgpu::RenderPipeline,

    // Grid
    grid_enabled: bool,
    grid_vertices: Vec<LineVertex>,
    grid_vertex_buffer: wgpu::Buffer,
    grid_index_buffer: wgpu::Buffer,
    grid_indices_len: u32,
    grid_render_pipeline: wgpu::RenderPipeline,
}

impl<W: World> AppImpl<'_, W> {
    #[inline]
    pub async fn new(
        configs: AppConfigs,
        mut world: W,
        event_loop: &ActiveEventLoop,
    ) -> anyhow::Result<Self> {
        let world_image = world.init_image();
        let world_aspect = world_image.width() as f32 / world_image.height() as f32;

        let update_interval = { Duration::from_secs(1) / configs.updates_per_second };

        let (window, window_size) = {
            let window = event_loop.create_window(configs.window_attributes.clone())?;
            let size = window.inner_size();
            (Arc::new(window), size)
        };

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&Default::default())
            .await
            .context("adapter not found")?;

        let surface = instance.create_surface(Arc::clone(&window))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Main Device"),
                    required_features: wgpu::Features::empty(),
                    ..Default::default()
                },
                None,
            )
            .await?;

        let surface_config = {
            let surface_caps = surface.get_capabilities(&adapter);
            let surface_format = surface_caps
                .formats
                .iter()
                .find(|f| f.is_srgb())
                .copied()
                .unwrap_or(surface_caps.formats[0]);

            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: surface_format,
                width: window_size.width,
                height: window_size.height,
                present_mode: surface_caps.present_modes[0],
                alpha_mode: surface_caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };
            surface.configure(&device, &config);
            config
        };

        let (texture, texture_view, texture_sampler) =
            world_image.create_texture(&device, &queue, Some("World Main Texture"))?;
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture_bind_group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture_sampler),
                },
            ],
        });

        let grid_vertices_len = (world_image.width() + world_image.height() + 2) * 4;
        let mut grid_vertices = vec![LineVertex::default(); grid_vertices_len as _];

        let (vertices, bounds) = aspect_adjusted_vertices(
            world_aspect,
            window_size,
            world_image.width(),
            world_image.height(),
            &mut grid_vertices,
        );

        // We use wgpu::IndexFormat::Uint16
        #[rustfmt::skip]
        let indices: [u16; 6] = [
            0, 1, 2,
            2, 1, 3
        ];
        let indices_len = indices.len() as u32;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            });
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Main Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("main.wgsl").into()),
            });

            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::desc()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_config.format,
                        blend: Some(wgpu::BlendState {
                            alpha: wgpu::BlendComponent::REPLACE,
                            color: wgpu::BlendComponent::REPLACE,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            })
        };

        let grid_indices = grid_indices(world_image.width(), world_image.height());
        let grid_indices_len = grid_indices.len() as u32;

        let grid_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Vertex Buffer"),
            contents: bytemuck::cast_slice(&grid_vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let grid_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Index Buffer"),
            contents: bytemuck::cast_slice(&grid_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let grid_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Grid Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Main Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("grid.wgsl").into()),
            });

            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Grid Render Pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[LineVertex::desc()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_config.format,
                        blend: Some(wgpu::BlendState {
                            alpha: wgpu::BlendComponent::REPLACE,
                            color: wgpu::BlendComponent::REPLACE,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            })
        };

        Ok(Self {
            configs,
            world,
            world_image,
            world_aspect,
            window,
            window_size,
            update_interval,
            last_update: Instant::now(),
            bounds,
            cursor_translated: None,
            paused: false,
            surface,
            device,
            queue,
            surface_config,
            should_update_texture: false,
            texture,
            texture_view,
            texture_sampler,
            texture_bind_group,
            vertex_buffer,
            index_buffer,
            indices_len,
            render_pipeline,
            grid_enabled: false,
            grid_vertices,
            grid_vertex_buffer,
            grid_index_buffer,
            grid_indices_len,
            grid_render_pipeline,
        })
    }

    #[inline]
    pub fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) -> anyhow::Result<()> {
        match event {
            WindowEvent::Resized(physical_size) => {
                self.resize(physical_size);
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.update();
                self.render().unwrap();
                self.window.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.keyboard_input(event);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.mouse_input(state, button);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_moved(position);
            }
            _ => (),
        }
        Ok(())
    }

    fn resize(&mut self, new_window_size: PhysicalSize<u32>) {
        if new_window_size == self.window_size {
            return;
        }
        self.window_size = new_window_size;
        if new_window_size.width == 0 || new_window_size.height == 0 {
            return;
        }

        // Update state
        self.surface_config.width = new_window_size.width;
        self.surface_config.height = new_window_size.height;
        self.surface.configure(&self.device, &self.surface_config);

        // Update vertex
        let (vertices, bounds) = aspect_adjusted_vertices(
            self.world_aspect,
            self.window_size,
            self.world_image.width(),
            self.world_image.height(),
            &mut self.grid_vertices,
        );

        self.queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        self.queue.write_buffer(
            &self.grid_vertex_buffer,
            0,
            bytemuck::cast_slice(&self.grid_vertices),
        );
        self.bounds = bounds;
    }

    fn update(&mut self) {
        let now = Instant::now();
        let dt = now - self.last_update;
        if dt < self.update_interval {
            return;
        }

        self.last_update = self
            .last_update
            .checked_add(self.update_interval)
            .unwrap_or(now);

        if !self.paused {
            self.run_update();
        }
    }

    fn run_update(&mut self) {
        self.world.update(&mut self.world_image);
        self.should_update_texture = true;
    }

    fn render(&mut self) -> anyhow::Result<()> {
        if self.should_update_texture {
            self.world_image
                .update_wgpu_texture(&self.texture, &self.queue);
            self.should_update_texture = false;
        }

        let output = self.surface.get_current_texture()?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.indices_len, 0, 0..1);
        }
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Grid Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.grid_render_pipeline);
            render_pass.set_vertex_buffer(0, self.grid_vertex_buffer.slice(..));
            render_pass
                .set_index_buffer(self.grid_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(
                grid_indices_range(self.grid_indices_len, self.grid_enabled),
                0,
                0..1,
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn keyboard_input(&mut self, event: KeyEvent) {
        use crate::util::is_pressed;

        if let Some(key) = self.configs.key_play {
            if is_pressed(&event, key) {
                self.paused = !self.paused;
            }
        }
        if self.paused {
            if let Some(key) = self.configs.key_update_once {
                if is_pressed(&event, key) {
                    self.run_update();
                }
            }
        }
        if let Some(key) = self.configs.key_grid {
            if is_pressed(&event, key) {
                self.grid_enabled = !self.grid_enabled;
            }
        }

        self.world.keyboard_input(event, &mut self.world_image);
        self.should_update_texture = true;
    }

    fn mouse_input(&mut self, state: ElementState, button: MouseButton) {
        self.world.mouse_input(
            MouseEvent {
                state,
                button,
                pos: self.cursor_translated,
            },
            &mut self.world_image,
        );
        self.should_update_texture = true;
    }

    fn cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        let mut pos = self.bounds.translate_position(position);

        // bounds check

        if let Some((x, y)) = pos {
            if x >= self.world_image.width() || y >= self.world_image.height() {
                pos = None;
            }
        }

        self.cursor_translated = pos;

        self.world
            .cursor_moved(self.cursor_translated, &mut self.world_image);

        self.should_update_texture = true; // This is bad
    }
}

#[derive(Debug)]
struct WorldTransform {
    min: (f64, f64),
    _max: (f64, f64),
    cell_scale: (f64, f64),
}

impl WorldTransform {
    fn translate_position(&self, pos: PhysicalPosition<f64>) -> Option<(u32, u32)> {
        fn calc_pos(val: f64, min: f64, scale: f64) -> Option<u32> {
            let val = val - min;
            (val >= 0.0).then(|| (val / scale) as _)
        }
        let x = calc_pos(pos.x, self.min.0, self.cell_scale.0)?;
        let y = calc_pos(pos.y, self.min.1, self.cell_scale.1)?;
        Some((x, y))
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
    ];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
struct LineVertex {
    position: [f32; 2],
    strength: f32,
}

impl LineVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32,
    ];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

fn aspect_adjusted_vertices(
    world_aspect: f32,
    window_size: PhysicalSize<u32>,
    world_width: u32,
    world_height: u32,
    grid_vertices: &mut [LineVertex],
) -> ([Vertex; 4], WorldTransform) {
    let (x, y) = {
        let window_aspect = window_size.width as f32 / window_size.height as f32;
        let (x, y) = if window_aspect > world_aspect {
            (world_aspect / window_aspect, 1.0)
        } else {
            (1.0, window_aspect / world_aspect)
        };
        // add margin
        let p = 0.999;
        let x = x * p;
        let y = y * p;
        (x, y)
    };

    let vertices = vertices_rectangle([-x, y], [x, -y]);

    // Calculate bounds
    let w = window_size.width as f64;
    let h = window_size.height as f64;
    let x0 = w * (1.0 - x as f64) / 2.0;
    let y0 = h * (1.0 - y as f64) / 2.0;
    let x1 = w - x0;
    let y1 = h - y0;
    let w1 = (x1 - x0) / world_width as f64;
    let h1 = (y1 - y0) / world_height as f64;
    let bounds = WorldTransform {
        min: (x0, y0),
        _max: (x1, y1),
        cell_scale: (w1, h1),
    };

    // Update grid info
    update_grid_vertices(
        grid_vertices,
        x,
        y,
        world_width,
        world_height,
        1.0 / window_size.width as f32,
        1.0 / window_size.height as f32,
    );

    (vertices, bounds)
}

fn vertices_rectangle(top_left: [f32; 2], bottom_right: [f32; 2]) -> [Vertex; 4] {
    let [a, b, c, d] = positions_rectangle(top_left, bottom_right);

    [
        Vertex {
            position: a,
            tex_coords: [0.0, 1.0],
        },
        Vertex {
            position: b,
            tex_coords: [1.0, 1.0],
        },
        Vertex {
            position: c,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: d,
            tex_coords: [1.0, 0.0],
        },
    ]
}

fn line_vertices_rectangle(
    top_left: [f32; 2],
    bottom_right: [f32; 2],
    strength: f32,
) -> [LineVertex; 4] {
    let [a, b, c, d] = positions_rectangle(top_left, bottom_right);

    [
        LineVertex {
            position: a,
            strength,
        },
        LineVertex {
            position: b,
            strength,
        },
        LineVertex {
            position: c,
            strength,
        },
        LineVertex {
            position: d,
            strength,
        },
    ]
}

fn positions_rectangle(top_left: [f32; 2], bottom_right: [f32; 2]) -> [[f32; 2]; 4] {
    let [x0, y0] = top_left;
    let [x1, y1] = bottom_right;

    // top_left
    // -1, 1
    //
    //        1, -1,
    //        bottom_right

    [[x0, y1], [x1, y1], [x0, y0], [x1, y0]]
}

fn update_grid_vertices(
    grid_vertices: &mut [LineVertex],
    x: f32,
    y: f32,
    world_width: u32,
    world_height: u32,
    half_line_width: f32,
    half_line_height: f32,
) {
    let x0 = -x;
    let y0 = -y;
    let x1 = x;
    let y1 = y;

    let w = world_width as f32;
    let h = world_height as f32;

    let vertical = |x: u32, strength: f32| {
        let p0 = (world_width - x) as f32 / w;
        let p1 = x as f32 / w;
        let lx = x0 * p0 + x1 * p1;
        line_vertices_rectangle(
            [lx - half_line_width, y1],
            [lx + half_line_width, y0],
            strength,
        )
    };
    let horizontal = |y: u32, strength: f32| {
        let p0 = (world_height - y) as f32 / h;
        let p1 = y as f32 / h;
        let ly = y0 * p0 + y1 * p1;
        line_vertices_rectangle(
            [x0, ly + half_line_height],
            [x1, ly - half_line_height],
            strength,
        )
    };
    let mut copy_vertices = |i: usize, vertices: [LineVertex; 4]| {
        let i = i * 4;
        grid_vertices[i..i + 4].copy_from_slice(&vertices);
    };

    copy_vertices(0, vertical(0, 1.0));
    copy_vertices(1, vertical(world_width, 1.0));
    copy_vertices(2, horizontal(0, 1.0));
    copy_vertices(3, horizontal(world_height, 1.0));

    for x in 1..world_width {
        copy_vertices(x as usize + 3, vertical(x, 0.5));
    }
    for y in 1..world_height {
        copy_vertices((y + world_width) as usize + 2, horizontal(y, 0.5));
    }
}

fn grid_indices(world_width: u32, world_height: u32) -> Vec<u32> {
    (0..world_width + world_height + 2)
        .flat_map(|i| {
            let i = i * 4;
            [i, i + 1, i + 2, i + 2, i + 1, i + 3]
        })
        .collect()
}

fn grid_indices_range(n_indices: u32, grid_enabled: bool) -> std::ops::Range<u32> {
    if grid_enabled {
        0..n_indices
    } else {
        0..24 // 6 * 4
    }
}
