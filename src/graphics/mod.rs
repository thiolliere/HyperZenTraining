use vulkano::device::{Device, Queue, DeviceExtensions};
use vulkano::swapchain::{self, Swapchain};
use vulkano::sampler::{Sampler, Filter, SamplerAddressMode, MipmapMode,
                       UnnormalizedSamplerAddressMode};
use vulkano::image::{SwapchainImage, AttachmentImage, ImmutableImage, ImageUsage, Dimensions};
// TODO: replace CpuAccessible by something else ?
use vulkano::buffer::{CpuAccessibleBuffer, ImmutableBuffer, CpuBufferPool, BufferUsage,
                      DeviceLocalBuffer};
use vulkano::framebuffer::{RenderPassDesc, RenderPass, Framebuffer, Subpass};
use vulkano::pipeline::{GraphicsPipeline, ComputePipeline};
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::pipeline::viewport::Viewport;
use vulkano::descriptor::PipelineLayoutAbstract;
use vulkano::descriptor::pipeline_layout::PipelineLayout;
use vulkano::descriptor::descriptor_set::{PersistentDescriptorSet, PersistentDescriptorSetImg,
                                          PersistentDescriptorSetSampler,
                                          PersistentDescriptorSetBuf, FixedSizeDescriptorSetsPool};
use vulkano::instance::PhysicalDevice;
use vulkano::format;
use vulkano::sync::GpuFuture;


use std::sync::Arc;
use std::iter;
use std::fs::File;

pub mod shader;
pub mod render_pass;
mod primitives;
mod colors;

pub use self::primitives::primitive;
pub use self::colors::color;

lazy_static! {
    pub static ref GROUP_COUNTER: GroupCounter = GroupCounter::new();
}

pub const GROUP_COUNTER_SIZE: usize = 65536;

pub struct GroupCounter {
    counter: ::std::sync::atomic::AtomicUsize,
}

impl GroupCounter {
    fn new() -> Self {
        GroupCounter { counter: ::std::sync::atomic::AtomicUsize::new(1) }
    }

    pub fn next(&self) -> u16 {
        self.counter.fetch_add(
            1,
            ::std::sync::atomic::Ordering::Relaxed,
        ) as u16
    }
}

#[derive(Debug, Clone)]
pub struct Vertex {
    position: [f32; 3],
}
impl_vertex!(Vertex, position);

#[derive(Debug, Clone)]
pub struct SecondVertex {
    position: [f32; 2],
}
impl_vertex!(SecondVertex, position);

#[derive(Debug, Clone)]
pub struct SecondVertexImgui {
    pos: [f32; 2],
    uv: [f32; 2],
    col: [f32; 4],
}
impl_vertex!(SecondVertexImgui, pos, uv, col);
impl From<::imgui::ImDrawVert> for SecondVertexImgui {
    fn from(vertex: ::imgui::ImDrawVert) -> Self {
        let r = (vertex.col >> 24) as u8 as f32;
        let g = (vertex.col >> 16) as u8 as f32;
        let b = (vertex.col >> 8) as u8 as f32;
        let a = vertex.col as u8 as f32;
        SecondVertexImgui {
            pos: [vertex.pos.x, vertex.pos.y],
            uv: [vertex.uv.x, vertex.uv.y],
            col: [r, g, b, a],
        }
    }
}

#[derive(Clone)]
pub struct Data {
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,

    pub swapchain: Arc<Swapchain>,
    pub images: Vec<Arc<SwapchainImage>>,
    pub width: u32,
    pub height: u32,

    pub primitives_vertex_buffers: Vec<Arc<ImmutableBuffer<[Vertex]>>>,
    pub fullscreen_vertex_buffer: Arc<ImmutableBuffer<[SecondVertex]>>,
    pub cursor_vertex_buffer: Arc<ImmutableBuffer<[SecondVertex]>>,

    pub view_uniform_buffer: CpuBufferPool<::graphics::shader::draw1_vs::ty::View>,
    pub world_uniform_buffer: CpuBufferPool<::graphics::shader::draw1_vs::ty::World>,
    pub tmp_erased_buffer: Arc<DeviceLocalBuffer<[u32; 65536]>>,
    pub erased_buffer: Arc<CpuAccessibleBuffer<[f32; 65536]>>,

    pub render_pass: Arc<RenderPass<render_pass::CustomRenderPassDesc>>,
    pub second_render_pass: Arc<RenderPass<render_pass::SecondCustomRenderPassDesc>>,

    pub framebuffer: Arc<Framebuffer<Arc<RenderPass<render_pass::CustomRenderPassDesc>>, ((((), Arc<AttachmentImage>), Arc<AttachmentImage>), Arc<AttachmentImage>)>>,
    pub second_framebuffers: Vec<Arc<Framebuffer<Arc<RenderPass<render_pass::SecondCustomRenderPassDesc>>, ((), Arc<SwapchainImage>)>>>,

    pub draw1_pipeline: Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>, Box<PipelineLayoutAbstract + Sync + Send>, ::Arc<RenderPass<render_pass::CustomRenderPassDesc>>>>,
    pub draw1_eraser_pipeline: Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>, Box<PipelineLayoutAbstract + Sync + Send>, ::Arc<RenderPass<render_pass::CustomRenderPassDesc>>>>,
    pub eraser1_pipeline: Arc<ComputePipeline<PipelineLayout<::graphics::shader::eraser1_cs::Layout>>>,
    pub eraser2_pipeline: Arc<ComputePipeline<PipelineLayout<::graphics::shader::eraser2_cs::Layout>>>,
    pub draw2_pipeline: Arc<GraphicsPipeline<SingleBufferDefinition<SecondVertex>, Box<PipelineLayoutAbstract + Sync + Send>, ::Arc<RenderPass<render_pass::SecondCustomRenderPassDesc>>>>,
    pub cursor_pipeline: Arc<GraphicsPipeline<SingleBufferDefinition<SecondVertex>, Box<PipelineLayoutAbstract + Sync + Send>, ::Arc<RenderPass<render_pass::SecondCustomRenderPassDesc>>>>,
    pub imgui_pipeline: Arc<GraphicsPipeline<SingleBufferDefinition<SecondVertexImgui>, Box<PipelineLayoutAbstract + Sync + Send>, ::Arc<RenderPass<render_pass::SecondCustomRenderPassDesc>>>>,

    pub draw1_view_descriptor_set_pool: FixedSizeDescriptorSetsPool<Arc<GraphicsPipeline<SingleBufferDefinition<::graphics::Vertex>, Box<PipelineLayoutAbstract + Sync + Send>, Arc<RenderPass<render_pass::CustomRenderPassDesc>>>>>,
    pub draw1_dynamic_descriptor_set_pool: FixedSizeDescriptorSetsPool<Arc<GraphicsPipeline<SingleBufferDefinition<::graphics::Vertex>, Box<PipelineLayoutAbstract + Sync + Send>, Arc<RenderPass<render_pass::CustomRenderPassDesc>>>>>,
    pub imgui_matrix_descriptor_set_pool: FixedSizeDescriptorSetsPool<Arc<GraphicsPipeline<SingleBufferDefinition<::graphics::SecondVertexImgui>, Box<PipelineLayoutAbstract + Sync + Send>, Arc<RenderPass<render_pass::SecondCustomRenderPassDesc>>>>>,

    pub cursor_descriptor_set: Arc<PersistentDescriptorSet<Arc<GraphicsPipeline<SingleBufferDefinition<::graphics::SecondVertex>, Box<PipelineLayoutAbstract + Sync + Send>, Arc<RenderPass<::graphics::render_pass::SecondCustomRenderPassDesc>>>>, (((), PersistentDescriptorSetImg<Arc<ImmutableImage<format::R8G8B8A8Unorm>>>), PersistentDescriptorSetSampler)>>,
    pub imgui_descriptor_set: Arc<PersistentDescriptorSet<Arc<GraphicsPipeline<SingleBufferDefinition<::graphics::SecondVertexImgui>, Box<PipelineLayoutAbstract + Sync + Send>, Arc<RenderPass<::graphics::render_pass::SecondCustomRenderPassDesc>>>>, (((), PersistentDescriptorSetImg<Arc<ImmutableImage<format::R8G8B8A8Unorm>>>), PersistentDescriptorSetSampler)>>,
    pub eraser1_descriptor_set: Arc<PersistentDescriptorSet<Arc<ComputePipeline<PipelineLayout<::graphics::shader::eraser1_cs::Layout>>>, ((((((), PersistentDescriptorSetImg<Arc<AttachmentImage>>), PersistentDescriptorSetSampler), PersistentDescriptorSetImg<Arc<AttachmentImage>>), PersistentDescriptorSetSampler), PersistentDescriptorSetBuf<Arc<DeviceLocalBuffer<[u32; 65536]>>>)>>,
    pub eraser2_descriptor_set: Arc<PersistentDescriptorSet<Arc<ComputePipeline<PipelineLayout<::graphics::shader::eraser2_cs::Layout>>>, (((), PersistentDescriptorSetBuf<Arc<DeviceLocalBuffer<[u32; 65536]>>>), PersistentDescriptorSetBuf<Arc<CpuAccessibleBuffer<[f32; 65536]>>>)>>,
    pub draw2_descriptor_set: Arc<PersistentDescriptorSet<Arc<GraphicsPipeline<SingleBufferDefinition<::graphics::SecondVertex>, Box<PipelineLayoutAbstract + Sync + Send>, Arc<RenderPass<render_pass::SecondCustomRenderPassDesc>>>>, (((((), PersistentDescriptorSetImg<Arc<AttachmentImage>>), PersistentDescriptorSetSampler), PersistentDescriptorSetBuf<Arc<ImmutableBuffer<[[f32; 4]]>>>), PersistentDescriptorSetBuf<Arc<CpuAccessibleBuffer<[f32; 65536]>>>)>>,
}

pub struct Graphics<'a> {
    pub physical: PhysicalDevice<'a>,
    pub data: Data,
}

impl<'a> Graphics<'a> {
    pub fn new(window: &'a ::vulkano_win::Window, imgui: &mut ::imgui::ImGui) -> Graphics<'a> {
        // TODO: read config and save device
        let physical = PhysicalDevice::enumerate(&window.surface().instance())
            .next()
            .expect("no device available");

        let queue_family = physical
            .queue_families()
            .find(|&q| {
                q.supports_graphics() && q.supports_compute() &&
                    window.surface().is_supported(q).unwrap_or(false)
            })
            .expect("couldn't find a graphical queue family");

        let (device, mut queues) = {
            let device_ext = DeviceExtensions {
                khr_swapchain: true,
                ..DeviceExtensions::none()
            };

            Device::new(
                physical,
                physical.supported_features(),
                &device_ext,
                [(queue_family, 0.5)].iter().cloned(),
            ).expect("failed to create device")
        };

        let queue = queues.next().unwrap();

        let (swapchain, images) = {
            let caps = window.surface().capabilities(physical).expect(
                "failed to get surface capabilities",
            );

            let dimensions = caps.current_extent.unwrap_or([1280, 1024]);
            let image_usage = ImageUsage {
                sampled: true,
                color_attachment: true,
                ..ImageUsage::none()
            };

            Swapchain::new(
                device.clone(),
                window.surface().clone(),
                caps.min_image_count,
                format::Format::B8G8R8A8Srgb,
                dimensions,
                1,
                image_usage,
                &queue,
                swapchain::SurfaceTransform::Identity,
                swapchain::CompositeAlpha::Opaque,
                swapchain::PresentMode::Fifo,
                true,
                None,
            ).expect("failed to create swapchain")
        };

        let width = images[0].dimensions()[0];
        let height = images[0].dimensions()[1];

        let depth_buffer_attachment = AttachmentImage::transient(
            device.clone(),
            images[0].dimensions(),
            format::Format::D16Unorm,
        ).unwrap();

        let tmp_image_attachment = {
            let usage = ImageUsage {
                color_attachment: true,
                sampled: true,
                ..ImageUsage::none()
            };
            AttachmentImage::with_usage(
                device.clone(),
                images[0].dimensions(),
                format::Format::R8G8B8A8Uint,
                usage,
            ).unwrap()
        };

        let tmp_erase_image_attachment = {
            let usage = ImageUsage {
                color_attachment: true,
                sampled: true,
                ..ImageUsage::none()
            };
            AttachmentImage::with_usage(
                device.clone(),
                images[0].dimensions(),
                format::Format::R8Uint,
                usage,
            ).unwrap()
        };

        let (primitives_vertex_buffers, mut futures) =
            primitives::instance_primitives(queue.clone());

        let (fullscreen_vertex_buffer, mut fullscreen_vertex_buffer_future) =
            ImmutableBuffer::from_iter(
                [
                    SecondVertex { position: [-1.0f32, -1.0] },
                    SecondVertex { position: [1.0, -1.0] },
                    SecondVertex { position: [-1.0, 1.0] },
                    SecondVertex { position: [1.0, 1.0] },
                    SecondVertex { position: [-1.0, 1.0] },
                    SecondVertex { position: [1.0, -1.0] },
                ].iter()
                    .cloned(),
                BufferUsage::vertex_buffer(),
                queue.clone(),
            ).expect("failed to create buffer");

        let (cursor_vertex_buffer, mut cursor_vertex_buffer_future) =
            ImmutableBuffer::from_iter(
                [
                    SecondVertex { position: [-0.5f32, -0.5] },
                    SecondVertex { position: [0.5, -0.5] },
                    SecondVertex { position: [-0.5, 0.5] },
                    SecondVertex { position: [0.5, 0.5] },
                    SecondVertex { position: [-0.5, 0.5] },
                    SecondVertex { position: [0.5, -0.5] },
                ].iter()
                    .cloned(),
                BufferUsage::vertex_buffer(),
                queue.clone(),
            ).expect("failed to create buffer");

        let (cursor_texture, mut cursor_tex_future) = {
            let file = File::open("assets/cursor.png").unwrap();
            let (info, mut reader) = ::png::Decoder::new(file).read_info().unwrap();
            assert_eq!(info.color_type, ::png::ColorType::RGBA);
            let mut buf = vec![0; info.buffer_size()];
            reader.next_frame(&mut buf).unwrap();

            ImmutableImage::from_iter(
                buf.into_iter(),
                Dimensions::Dim2d {
                    width: info.width,
                    height: info.height,
                },
                format::R8G8B8A8Unorm,
                queue.clone(),
            ).unwrap()
        };

        let cursor_sampler = Sampler::new(
            device.clone(),
            Filter::Linear,
            Filter::Linear,
            MipmapMode::Nearest,
            SamplerAddressMode::ClampToEdge,
            SamplerAddressMode::ClampToEdge,
            SamplerAddressMode::ClampToEdge,
            // TODO: What values here
            0.0,
            1.0,
            0.0,
            0.0,
        ).unwrap();

        let cursor_tex_dim = cursor_texture.dimensions();

        let draw1_vs =
            shader::draw1_vs::Shader::load(device.clone()).expect("failed to create shader module");
        let draw1_fs =
            shader::draw1_fs::Shader::load(device.clone()).expect("failed to create shader module");
        let draw1_eraser_fs =
            shader::draw1_eraser_fs::Shader::load(device.clone()).expect("failed to create shader module");

        let eraser1_cs = shader::eraser1_cs::Shader::load(device.clone()).expect(
            "failed to create shader module",
        );
        let eraser2_cs = shader::eraser2_cs::Shader::load(device.clone()).expect(
            "failed to create shader module",
        );

        let draw2_vs =
            shader::draw2_vs::Shader::load(device.clone()).expect("failed to create shader module");
        let draw2_fs =
            shader::draw2_fs::Shader::load(device.clone()).expect("failed to create shader module");

        let cursor_vs = shader::cursor_vs::Shader::load(device.clone()).expect(
            "failed to create shader module",
        );
        let cursor_fs = shader::cursor_fs::Shader::load(device.clone()).expect(
            "failed to create shader module",
        );

        let imgui_vs =
            shader::imgui_vs::Shader::load(device.clone()).expect("failed to create shader module");
        let imgui_fs =
            shader::imgui_fs::Shader::load(device.clone()).expect("failed to create shader module");

        let render_pass = Arc::new(
            render_pass::CustomRenderPassDesc
                .build_render_pass(device.clone())
                .unwrap(),
        );

        let second_render_pass = Arc::new(
            render_pass::SecondCustomRenderPassDesc
                .build_render_pass(device.clone())
                .unwrap(),
        );

        let draw1_pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<Vertex>()
                .vertex_shader(draw1_vs.main_entry_point(), ())
                .viewports(iter::once(Viewport {
                    origin: [0.0, 0.0],
                    depth_range: 0.0..1.0,
                    dimensions: [width as f32, height as f32],
                }))
                .fragment_shader(draw1_fs.main_entry_point(), ())
                .depth_stencil_simple_depth()
                .sample_shading_enabled(1.0)
                .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
                .build(device.clone())
                .unwrap(),
        );

        let draw1_eraser_pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<Vertex>()
                .vertex_shader(draw1_vs.main_entry_point(), ())
                .viewports(iter::once(Viewport {
                    origin: [0.0, 0.0],
                    depth_range: 0.0..1.0,
                    dimensions: [width as f32, height as f32],
                }))
                .fragment_shader(draw1_eraser_fs.main_entry_point(), ())
                .depth_stencil_simple_depth()
                .sample_shading_enabled(1.0)
                .render_pass(Subpass::from(render_pass.clone(), 1).unwrap())
                .build(device.clone())
                .unwrap(),
        );

        let eraser1_pipeline =
            Arc::new(
                ComputePipeline::new(device.clone(), &eraser1_cs.main_entry_point(), &()).unwrap(),
            );
        let eraser2_pipeline =
            Arc::new(
                ComputePipeline::new(device.clone(), &eraser2_cs.main_entry_point(), &()).unwrap(),
            );

        let draw2_pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<SecondVertex>()
                .vertex_shader(draw2_vs.main_entry_point(), ())
                .triangle_list()
                .viewports(iter::once(Viewport {
                    origin: [0.0, 0.0],
                    depth_range: 0.0..1.0,
                    dimensions: [width as f32, height as f32],
                }))
                .fragment_shader(draw2_fs.main_entry_point(), ())
                .render_pass(Subpass::from(second_render_pass.clone(), 0).unwrap())
                .build(device.clone())
                .unwrap(),
        );

        let cursor_pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<SecondVertex>()
                .vertex_shader(cursor_vs.main_entry_point(), ())
                .triangle_list()
                .viewports(iter::once(Viewport {
                    origin: [
                        (width - cursor_tex_dim.width() * 2) as f32 / 2.0,
                        (height - cursor_tex_dim.height() * 2) as f32 / 2.0,
                    ],
                    depth_range: 0.0..1.0,
                    dimensions: [
                        (cursor_tex_dim.width() * 2) as f32,
                        (cursor_tex_dim.width() * 2) as f32,
                    ],
                }))
                .fragment_shader(cursor_fs.main_entry_point(), ())
                .blend_alpha_blending()
                .render_pass(Subpass::from(second_render_pass.clone(), 0).unwrap())
                .build(device.clone())
                .unwrap(),
        );

        let imgui_pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<SecondVertexImgui>()
                .vertex_shader(imgui_vs.main_entry_point(), ())
                .triangle_list()
                .viewports_fixed_scissors_dynamic(iter::once(Viewport {
                    origin: [0.0, 0.0],
                    depth_range: 0.0..1.0,
                    dimensions: [width as f32, height as f32],
                }))
                .fragment_shader(imgui_fs.main_entry_point(), ())
                .blend_alpha_blending()
                .render_pass(Subpass::from(second_render_pass.clone(), 0).unwrap())
                .build(device.clone())
                .unwrap(),
        );

        let framebuffer = Arc::new(
            Framebuffer::start(render_pass.clone())
                .add(tmp_image_attachment.clone())
                .unwrap()
                .add(tmp_erase_image_attachment.clone())
                .unwrap()
                .add(depth_buffer_attachment.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        let second_framebuffers = images
            .iter()
            .map(|image| {
                Arc::new(
                    Framebuffer::start(second_render_pass.clone())
                        .add(image.clone())
                        .unwrap()
                        .build()
                        .unwrap(),
                )
            })
            .collect::<Vec<_>>();

        let view_uniform_buffer = CpuBufferPool::<::graphics::shader::draw1_vs::ty::View>::new(
            device.clone(),
            BufferUsage::uniform_buffer(),
        );

        let world_uniform_buffer = CpuBufferPool::<::graphics::shader::draw1_vs::ty::World>::new(
            device.clone(),
            BufferUsage::uniform_buffer(),
        );

        let (colors_buffer, mut colors_buf_future) = {
            let colors = colors::colors();
            ImmutableBuffer::from_iter(
                colors.into_iter(),
                // TODO: not all buffer usage
                BufferUsage::all(),
                queue.clone(),
            ).unwrap()
        };

        let (imgui_texture, mut imgui_tex_future) = imgui
            .prepare_texture(|handle| {
                ImmutableImage::from_iter(
                    handle.pixels.iter().cloned(),
                    Dimensions::Dim2d {
                        width: handle.width,
                        height: handle.height,
                    },
                    format::R8G8B8A8Unorm,
                    queue.clone(),
                )
            })
            .unwrap();

        // TODO: not all buffer usage
        let tmp_erased_buffer = DeviceLocalBuffer::<[u32; GROUP_COUNTER_SIZE]>::new(
            device.clone(),
            BufferUsage::all(),
            vec![queue.family()].into_iter(),
        ).unwrap();
        let erased_buffer = CpuAccessibleBuffer::from_data(
            device.clone(),
            BufferUsage::all(),
            [1f32; GROUP_COUNTER_SIZE],
        ).unwrap();

        let cursor_descriptor_set = Arc::new(
            PersistentDescriptorSet::start(cursor_pipeline.clone(), 0)
                .add_sampled_image(cursor_texture.clone(), cursor_sampler.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        let draw2_descriptor_set = Arc::new(
            PersistentDescriptorSet::start(draw2_pipeline.clone(), 0)
                .add_sampled_image(
                    tmp_image_attachment.clone(),
                    Sampler::unnormalized(
                        device.clone(),
                        Filter::Nearest,
                        UnnormalizedSamplerAddressMode::ClampToEdge,
                        UnnormalizedSamplerAddressMode::ClampToEdge,
                    ).unwrap(),
                )
                .unwrap()
                .add_buffer(colors_buffer.clone())
                .unwrap()
                .add_buffer(erased_buffer.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        let imgui_descriptor_set = {
            Arc::new(
                PersistentDescriptorSet::start(imgui_pipeline.clone(), 1)
                    .add_sampled_image(
                        imgui_texture,
                        Sampler::new(
                            device.clone(),
                            Filter::Nearest, // TODO: linear or nearest
                            Filter::Linear, // TODO: linear or nearest
                            MipmapMode::Linear, // TODO: linear or nearest
                            SamplerAddressMode::MirroredRepeat,
                            SamplerAddressMode::MirroredRepeat,
                            SamplerAddressMode::MirroredRepeat,
                            0.0,
                            1.0,
                            0.0,
                            0.0,
                        ).unwrap(),
                    )
                    .unwrap()
                    .build()
                    .unwrap(),
            )
        };

        let draw1_view_descriptor_set_pool =
            FixedSizeDescriptorSetsPool::new(draw1_pipeline.clone(), 0);
        let draw1_dynamic_descriptor_set_pool =
            FixedSizeDescriptorSetsPool::new(draw1_pipeline.clone(), 0);
        let imgui_matrix_descriptor_set_pool =
            FixedSizeDescriptorSetsPool::new(imgui_pipeline.clone(), 0);


        let eraser1_descriptor_set = Arc::new(
            PersistentDescriptorSet::start(eraser1_pipeline.clone(), 0)
                .add_sampled_image(
                    tmp_image_attachment.clone(),
                    Sampler::unnormalized(
                        device.clone(),
                        Filter::Nearest,
                        UnnormalizedSamplerAddressMode::ClampToEdge,
                        UnnormalizedSamplerAddressMode::ClampToEdge,
                    ).unwrap(),
                )
                .unwrap()
                .add_sampled_image(
                    tmp_erase_image_attachment.clone(),
                    Sampler::unnormalized(
                        device.clone(),
                        Filter::Nearest,
                        UnnormalizedSamplerAddressMode::ClampToEdge,
                        UnnormalizedSamplerAddressMode::ClampToEdge,
                    ).unwrap(),
                )
                .unwrap()
                .add_buffer(tmp_erased_buffer.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        let eraser2_descriptor_set = Arc::new(
            PersistentDescriptorSet::start(eraser2_pipeline.clone(), 0)
                .add_buffer(tmp_erased_buffer.clone())
                .unwrap()
                .add_buffer(erased_buffer.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        // TODO: return this future to enforce it later ?
        // TODO: also is it supposed to be used that way ?
        //       it should be flush
        cursor_tex_future.cleanup_finished();
        colors_buf_future.cleanup_finished();
        imgui_tex_future.cleanup_finished();
        fullscreen_vertex_buffer_future.cleanup_finished();
        cursor_vertex_buffer_future.cleanup_finished();
        for future in &mut futures {
            future.cleanup_finished();
        }

        Graphics {
            physical,
            data: Data {
                fullscreen_vertex_buffer,
                swapchain,
                images,
                device,
                queue,
                render_pass,
                second_render_pass,
                draw1_pipeline,
                draw1_eraser_pipeline,
                draw2_pipeline,
                framebuffer,
                second_framebuffers,
                width,
                height,
                view_uniform_buffer,
                primitives_vertex_buffers,
                cursor_descriptor_set,
                cursor_pipeline,
                imgui_pipeline,
                cursor_vertex_buffer,
                imgui_descriptor_set,
                draw1_view_descriptor_set_pool,
                draw1_dynamic_descriptor_set_pool,
                imgui_matrix_descriptor_set_pool,
                eraser1_pipeline,
                eraser2_pipeline,
                eraser1_descriptor_set,
                eraser2_descriptor_set,
                tmp_erased_buffer,
                draw2_descriptor_set,
                world_uniform_buffer,
                erased_buffer,
            },
        }
    }
}
