use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ScreenSize(pub u32, pub u32);

impl UniformData for ScreenSize {}
pub type ScreenUniform = Uniform<ScreenSize>;

pub trait UniformData: Pod + Copy {
    fn create_uniform(&self, device: &Device, binding: u32, label: Option<&str>) -> Uniform<Self>
    where
        Self: Sized,
    {
        Uniform::new(device, binding, self, label)
    }
}

pub struct Uniform<T: bytemuck::Pod> {
    pub buffer: Buffer,
    pub bind_group: BindGroup,
    pub layout: BindGroupLayout,
    phantom: std::marker::PhantomData<T>,
}

impl<T: bytemuck::Pod> Uniform<T> {
    pub fn new(device: &wgpu::Device, binding: u32, data: &T, label: Option<&str>) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label,
            contents: bytemuck::bytes_of(data),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("uniform_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("uniform_bind_group"),
        });

        Self {
            buffer,
            bind_group,
            layout,
            phantom: std::marker::PhantomData,
        }
    }

    pub fn update(&self, queue: &wgpu::Queue, data: &T) {
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(data));
    }
}
