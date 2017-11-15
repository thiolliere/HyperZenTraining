use vulkano::device::Queue;
use vulkano::buffer::{ImmutableBuffer, BufferUsage};
use vulkano::sync::{GpuFuture, now};
use std::sync::Arc;
use super::Vertex;
use super::DebugVertex;
use wavefront_obj::obj;

use std::f32::consts::PI;

pub fn instance_primitives(
    queue: Arc<Queue>,
) -> (Vec<Vec<Arc<ImmutableBuffer<[Vertex]>>>>, Box<GpuFuture>) {
    let mut primitives_buffers_def = vec![];

    // Plane
    primitives_buffers_def.push(vec![
                 vec![
                 Vertex { position: [-1.0, -1.0, 0.0] },
                 Vertex { position: [1.0, -1.0, 0.0] },
                 Vertex { position: [-1.0, 1.0, 0.0] },
                 Vertex { position: [1.0, 1.0, 0.0] },
                 Vertex { position: [-1.0, 1.0, 0.0] },
                 Vertex { position: [1.0, -1.0, 0.0] },
                 ]
    ]);

    // Square pyramid
    primitives_buffers_def.push(
        vec![
            vec![
                Vertex { position: [-1.0, -1.0, -1.0] },
                Vertex { position: [1.0, -1.0, -1.0] },
                Vertex { position: [-1.0, 1.0, -1.0] },

                Vertex { position: [1.0, 1.0, -1.0] },
                Vertex { position: [1.0, -1.0, -1.0] },
                Vertex { position: [-1.0, 1.0, -1.0] },
            ],
            vec![
                Vertex { position: [-1.0, -1.0, -1.0] },
                Vertex { position: [-1.0, 1.0, -1.0] },
                Vertex { position: [0.0, 0.0, 1.0] },
            ],
            vec![
                Vertex { position: [-1.0, 1.0, -1.0] },
                Vertex { position: [1.0, 1.0, -1.0] },
                Vertex { position: [0.0, 0.0, 1.0] },
            ],
            vec![
                Vertex { position: [1.0, 1.0, -1.0] },
                Vertex { position: [1.0, -1.0, -1.0] },
                Vertex { position: [0.0, 0.0, 1.0] },
            ],
            vec![
                Vertex { position: [1.0, -1.0, -1.0] },
                Vertex { position: [-1.0, -1.0, -1.0] },
                Vertex { position: [0.0, 0.0, 1.0] },
            ],
        ]
    );

    primitives_buffers_def.push(
        vec![
            vec![
                Vertex { position: [-1.0, -0.86602540378443864676, -1.0] },
                Vertex { position: [0.0, 0.86602540378443864676, -1.0] },
                Vertex { position: [1.0, -0.86602540378443864676, -1.0] },
            ],
            vec![
                Vertex { position: [-1.0, -0.86602540378443864676, -1.0] },
                Vertex { position: [0.0, 0.86602540378443864676, -1.0] },
                Vertex { position: [0.0, 0.0, 1.0] },
            ],
            vec![
                Vertex { position: [0.0, 0.86602540378443864676, -1.0] },
                Vertex { position: [1.0, -0.86602540378443864676, -1.0] },
                Vertex { position: [0.0, 0.0, 1.0] },
            ],
            vec![
                Vertex { position: [-1.0, -0.86602540378443864676, -1.0] },
                Vertex { position: [1.0, -0.86602540378443864676, -1.0] },
                Vertex { position: [0.0, 0.0, 1.0] },
            ]
        ]
    );

    let sphere_vertices = {
        let sphere = ::ncollide::procedural::sphere(1.0, 16, 16, false);
        let indices = match sphere.indices {
            ::ncollide::procedural::IndexBuffer::Unified(ref indices) => indices.clone(),
            _ => unreachable!(),
        };

        let mut vertices = vec![];
        for p in indices {
            vertices.push(Vertex {
                position: [
                    sphere.coords[p.x as usize][0] * 2.0,
                    sphere.coords[p.x as usize][1] * 2.0,
                    sphere.coords[p.x as usize][2] * 2.0,
                ],
            });
            vertices.push(Vertex {
                position: [
                    sphere.coords[p.y as usize][0] * 2.0,
                    sphere.coords[p.y as usize][1] * 2.0,
                    sphere.coords[p.y as usize][2] * 2.0,
                ],
            });
            vertices.push(Vertex {
                position: [
                    sphere.coords[p.z as usize][0] * 2.0,
                    sphere.coords[p.z as usize][1] * 2.0,
                    sphere.coords[p.z as usize][2] * 2.0,
                ],
            });
        }

        vertices
    };

    primitives_buffers_def.push(vec![sphere_vertices]);

    let mut nine_buffers_def = vec![vec![], vec![]];
    for i in 0..9 {
        let a0 = i as f32 * 2.0*PI/9.0;
        let a1 = (i+1) as f32 * 2.0*PI/9.0;

        let p0 = [a0.cos(), a0.sin()];
        let p1 = [a1.cos(), a1.sin()];

        nine_buffers_def[0].push(Vertex { position: [p0[0], p0[1], -1.0]});
        nine_buffers_def[0].push(Vertex { position: [p1[0], p1[1], -1.0]});
        nine_buffers_def[0].push(Vertex { position: [0.0, 0.0, -1.0]});

        nine_buffers_def[1].push(Vertex { position: [p0[0], p0[1], -1.0]});
        nine_buffers_def[1].push(Vertex { position: [p1[0], p1[1], -1.0]});
        nine_buffers_def[1].push(Vertex { position: [p1[0], p1[1], 1.0]});

        nine_buffers_def.push(
            vec![
                Vertex { position: [p0[0], p0[1], -1.0]},
                Vertex { position: [p0[0], p0[1], 1.0]},
                Vertex { position: [p1[0], p1[1], 1.0]},

                Vertex { position: [p0[0], p0[1], 1.0]},
                Vertex { position: [p1[0], p1[1], 1.0]},
                Vertex { position: [0.0, 0.0, 1.0]},
            ]
        );
    }
    primitives_buffers_def.push(nine_buffers_def);

    let mut final_future = Box::new(now(queue.device().clone())) as Box<GpuFuture>;
    let mut primitives_buffers = vec![];
    for primitive_buffers_def in primitives_buffers_def {
        let mut primitive_buffers = vec![];
        for buffer_def in primitive_buffers_def {
            let (buffer, future) = ImmutableBuffer::from_iter(
                buffer_def.iter().cloned(),
                BufferUsage::vertex_buffer(),
                queue.clone(),
            ).expect("failed to create buffer");

            primitive_buffers.push(buffer);
            final_future = Box::new(final_future.join(future)) as Box<GpuFuture>;
        }
        primitives_buffers.push(primitive_buffers);
    }

    (primitives_buffers, final_future)
}

#[allow(unused)]
pub mod primitive {
    pub enum Primitive {
        Plane,
        SquarePyramid,
        TrianglePyramid,
        Sphere,
        Nine,
    }

    impl Primitive {
        pub fn instantiate(&self) -> (usize, Vec<u16>) {
            match *self {
                Primitive::Plane => (0, GROUP_COUNTER.instantiate(1)),
                Primitive::SquarePyramid => (1, GROUP_COUNTER.instantiate(5)),
                Primitive::TrianglePyramid => (2, GROUP_COUNTER.instantiate(4)),
                Primitive::Sphere => (3, GROUP_COUNTER.instantiate(1)),
                Primitive::Nine => (4, GROUP_COUNTER.instantiate(1)),
            }
        }
    }

    lazy_static! {
        static ref GROUP_COUNTER: GroupCounter = GroupCounter::new();
    }

    pub const GROUP_COUNTER_SIZE: usize = 65536;

    struct GroupCounter {
        counter: ::std::sync::atomic::AtomicUsize,
    }

    impl GroupCounter {
        fn new() -> Self {
            GroupCounter { counter: ::std::sync::atomic::AtomicUsize::new(1) }
        }

        fn next(&self) -> u16 {
            self.counter.fetch_add(
                1,
                ::std::sync::atomic::Ordering::Relaxed,
            ) as u16
        }

        fn instantiate(&self, n: usize) -> Vec<u16> {
            (0..n).map(|_| self.next()).collect()
        }
    }
}

pub fn load_debug_arrow(
    queue: Arc<Queue>,
) -> (Arc<ImmutableBuffer<[DebugVertex]>>, Box<GpuFuture>) {
    let arrow = obj::parse(include_str!("arrow.obj").into()).unwrap();

    let mut vertices = vec![];
    for object in &arrow.objects {
        assert!(object.geometry.len() == 1);
        for shape in &object.geometry[0].shapes {
            let indexes = match shape.primitive {
                obj::Primitive::Triangle(a, b, c) => [
                    (a.0, a.2.unwrap()),
                    (b.0, b.2.unwrap()),
                    (c.0, c.2.unwrap()),
                ],
                _ => panic!("arrow obj not handled"),
            };
            for &(v, n) in &indexes {
                vertices.push(DebugVertex {
                    position: [
                        object.vertices[v].x as f32,
                        object.vertices[v].y as f32,
                        object.vertices[v].z as f32,
                    ],
                    normal: [
                        object.normals[n].x as f32,
                        object.normals[n].y as f32,
                        object.normals[n].z as f32,
                    ],
                });
            }
        }
    }

    let res = ImmutableBuffer::from_iter(
        vertices.iter().cloned(),
        BufferUsage::vertex_buffer(),
        queue.clone(),
    ).expect("failed to create buffer");
    (res.0, Box::new(res.1) as Box<GpuFuture>)
}
