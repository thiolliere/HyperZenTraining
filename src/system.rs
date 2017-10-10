use winit::{Event, WindowEvent, ElementState, MouseButton};
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::buffer::{ImmutableBuffer, BufferUsage};
use vulkano::pipeline::viewport::Scissor;
use nphysics::object::WorldObject;
use util::Direction;
use specs::Join;
use alga::general::SubsetOf;
use util::{high_byte, low_byte};

use std::sync::Arc;
use std::cell::RefCell;

// TODO: get mouse from axis and check if there are differences because of acceleration
pub struct PlayerControlSystem {
    directions: Vec<::util::Direction>,
    pointer: [f32; 2],
}

impl PlayerControlSystem {
    pub fn new() -> Self {
        PlayerControlSystem {
            directions: vec![],
            pointer: [0.0, 0.0],
        }
    }
}

impl<'a> ::specs::System<'a> for PlayerControlSystem {
    type SystemData = (::specs::ReadStorage<'a, ::component::Player>,
     ::specs::WriteStorage<'a, ::component::Aim>,
     ::specs::WriteStorage<'a, ::component::Shooter>,
     ::specs::WriteStorage<'a, ::component::Momentum>,
     ::specs::Fetch<'a, ::resource::WinitEvents>,
     ::specs::Fetch<'a, ::resource::Graphics>,
     ::specs::Fetch<'a, ::resource::Config>);

    fn run(
        &mut self,
        (players, mut aims, mut shooters, mut momentums, events, graphics, config): Self::SystemData,
    ) {
        let (_, player_aim, player_shooter, player_momentum) =
            (&players, &mut aims, &mut shooters, &mut momentums)
                .join()
                .next()
                .unwrap();
        for ev in events.iter() {
            match *ev {
                Event::WindowEvent {
                    event: WindowEvent::MouseInput {
                        button: MouseButton::Left,
                        state,
                        ..
                    },
                    ..
                } => {
                    match state {
                        ElementState::Pressed => player_shooter.set_shoot(true),
                        ElementState::Released => player_shooter.set_shoot(false),
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::MouseMoved { position: (dx, dy), .. }, ..
                } => {
                    self.pointer[0] += (dx as f32 - graphics.width as f32 / 2.0) *
                        config.mouse_sensibility();
                    self.pointer[1] += (dy as f32 - graphics.height as f32 / 2.0) *
                        config.mouse_sensibility();
                    self.pointer[1] = self.pointer[1].min(::std::f32::consts::FRAC_PI_2).max(
                        -::std::f32::consts::FRAC_PI_2,
                    );
                }
                Event::WindowEvent { event: WindowEvent::KeyboardInput { input, .. }, .. } => {
                    let direction = match input.scancode {
                        25 => Some(Direction::Forward),
                        38 => Some(Direction::Left),
                        39 => Some(Direction::Backward),
                        40 => Some(Direction::Right),
                        _ => None,
                    };
                    if let Some(direction) = direction {
                        self.directions.retain(|&elt| elt != direction);
                        if let ElementState::Pressed = input.state {
                            self.directions.push(direction);
                        }
                    }
                }
                _ => (),
            }
        }

        player_aim.dir = ::na::Rotation3::new(::na::Vector3::new(0.0, 0.0, -self.pointer[0])) *
            ::na::Rotation3::new(::na::Vector3::new(0.0, self.pointer[1], 0.0)) *
            ::na::Vector3::x();
        player_aim.x_dir = self.pointer[0];

        let mut move_vector: ::na::Vector3<f32> = ::na::zero();
        if self.directions.is_empty() {
            player_momentum.direction = ::na::zero();
        } else {
            for &direction in &self.directions {
                match direction {
                    Direction::Forward => move_vector[0] = 1.0,
                    Direction::Backward => move_vector[0] = -1.0,
                    Direction::Left => move_vector[1] = 1.0,
                    Direction::Right => move_vector[1] = -1.0,
                }
            }
            move_vector = (::na::Rotation3::new(::na::Vector3::new(0.0, 0.0, -self.pointer[0])) *
                               move_vector)
                .normalize();
            player_momentum.direction = move_vector;
        }
    }
}

pub struct AvoiderControlSystem;

impl<'a> ::specs::System<'a> for AvoiderControlSystem {
    type SystemData = (::specs::ReadStorage<'a, ::component::Player>,
     ::specs::ReadStorage<'a, ::component::Aim>,
     ::specs::ReadStorage<'a, ::component::PhysicBody>,
     ::specs::WriteStorage<'a, ::component::Avoider>,
     ::specs::WriteStorage<'a, ::component::Momentum>,
     ::specs::Fetch<'a, ::resource::PhysicWorld>,
     ::specs::Fetch<'a, ::resource::Maze>);

    fn run(
        &mut self,
        (players, aims, bodies, mut avoiders, mut momentums, physic_world, maze): Self::SystemData,
    ) {
        let (_, player_aim, player_body) = (&players, &aims, &bodies).join().next().unwrap();

        let player_pos = player_body.get(&physic_world).position().clone();

        for (avoider, momentum, body) in (&mut avoiders, &mut momentums, &bodies).join() {
            let avoider_pos = body.get(&physic_world).position().clone();

            let recompute_goal = if let Some(goal) = avoider.goal {
                (avoider_pos.translation.vector -
                     ::na::Vector3::new(
                        goal.0 as f32 + 0.5,
                        goal.1 as f32 + 0.5,
                        avoider_pos.translation.vector[2],
                    )).norm() < 0.5
            } else {
                if (avoider_pos.translation.vector - player_pos.translation.vector).norm() < 1.0 {
                    avoider.goal.take();
                    false
                } else {
                    true
                }
            };

            if recompute_goal {
                let pos = (
                    avoider_pos.translation.vector[0] as usize,
                    avoider_pos.translation.vector[1] as usize,
                );
                let goal = (
                    player_pos.translation.vector[0] as usize,
                    player_pos.translation.vector[1] as usize,
                );
                avoider.goal = maze.find_path(pos, goal).unwrap().0.get(1).cloned();
            }

            let (goal_direction, goal_coef) = {
                let goal_pos = if let Some(goal) = avoider.goal {
                    ::na::Vector3::new(
                        goal.0 as f32 + 0.5,
                        goal.1 as f32 + 0.5,
                        avoider_pos.translation.vector[2],
                    )
                } else {
                    player_pos.translation.vector
                };

                (
                    (goal_pos - avoider_pos.translation.vector).normalize(),
                    1f32,
                )
            };

            let (avoid_direction, avoid_coef) = {
                let avoider_pos_rel_player = avoider_pos.translation.vector -
                    player_pos.translation.vector;
                let avoid_vector = avoider_pos_rel_player -
                    avoider_pos_rel_player.dot(&player_aim.dir) * player_aim.dir;
                if avoid_vector.norm() != 0.0 {
                    let avoid_norm = avoid_vector.norm();
                    let avoid_direction = avoid_vector.normalize();
                    if avoid_norm > 0.5 {
                        (avoid_direction, 0f32)
                    } else {
                        // TODO: coefficent
                        (avoid_direction, 1f32) //1.0/avoid_norm)
                    }
                } else {
                    let random = ::na::Vector3::new_random();
                    (
                        (random - random.dot(&player_aim.dir) * player_aim.dir).normalize(),
                        1f32,
                        // TODO: coefficient
                    ) //1000f32)
                }
            };

            momentum.direction = (goal_coef * goal_direction + avoid_coef * avoid_direction)
                .normalize();
        }
    }
}

pub struct BouncerControlSystem;

impl<'a> ::specs::System<'a> for BouncerControlSystem {
    type SystemData = (::specs::ReadStorage<'a, ::component::Contactor>,
     ::specs::ReadStorage<'a, ::component::Bouncer>,
     ::specs::WriteStorage<'a, ::component::Momentum>);

    fn run(&mut self, (contactors, bouncers, mut momentums): Self::SystemData) {
        for (_, momentum, contactor) in (&bouncers, &mut momentums, &contactors).join() {
            if contactor.contacts.len() == 0 {
                break;
            }

            let mut normal = ::na::Vector3::new(0.0, 0.0, 0.0);
            for &(_, ref contact) in &contactor.contacts {
                normal -= contact.depth * contact.normal;
            }
            normal.normalize_mut();
            let proj_on_normal = momentum.direction.dot(&normal) * normal;
            momentum.direction -= 2.0 * proj_on_normal;
        }
    }
}


pub struct PhysicSystem;

impl<'a> ::specs::System<'a> for PhysicSystem {
    type SystemData = (::specs::ReadStorage<'a, ::component::Player>,
     ::specs::ReadStorage<'a, ::component::Momentum>,
     ::specs::WriteStorage<'a, ::component::PhysicBody>,
     ::specs::WriteStorage<'a, ::component::Contactor>,
     ::specs::Fetch<'a, ::resource::Config>,
     ::specs::FetchMut<'a, ::resource::PhysicWorld>);

    fn run(
        &mut self,
        (player, momentums, mut bodies, mut contactors, config, mut physic_world): Self::SystemData,
    ) {
        for (momentum, body) in (&momentums, &mut bodies).join() {
            let body = body.get_mut(&mut physic_world);
            let lin_vel = body.lin_vel();
            let ang_vel = body.ang_vel();

            // TODO: use integrator to modify rigidbody
            body.clear_forces();
            body.append_lin_force(-momentum.damping * lin_vel);
            let direction_force = momentum.force * momentum.direction;
            if let Some(pnt_to_com) = momentum.pnt_to_com {
                let pnt_to_com = body.position().rotation * pnt_to_com;
                body.append_force_wrt_point(direction_force, pnt_to_com);
            } else {
                body.append_lin_force(direction_force);
            }
            body.set_ang_vel_internal(momentum.ang_damping * ang_vel);

            // TODO: gravity if not touching floor
            // body.append_lin_force(10.0*::na::Vector3::new(0.0,0.0,-1.0));
        }
        for _ in 0..2 {
            for contactor in (&mut contactors).join() {
                contactor.contacts.clear();
            }

            physic_world.step(config.dt().clone() / 2.);

            for (co1, co2, mut contact) in physic_world.collision_world().contacts() {
                match (&co1.data, &co2.data) {
                    (&WorldObject::RigidBody(co1), &WorldObject::RigidBody(co2)) => {
                        let body_1 = physic_world.rigid_body(co1);
                        let entity_1 = ::component::PhysicBody::entity(body_1);
                        let body_2 = physic_world.rigid_body(co2);
                        let entity_2 = ::component::PhysicBody::entity(body_2);

                        if let Some(contactor) = contactors.get_mut(entity_1) {
                            contactor.contacts.push((entity_2, contact.clone()));
                        }

                        if let Some(contactor) = contactors.get_mut(entity_2) {
                            contact.flip();
                            contactor.contacts.push((entity_1, contact));
                        }
                    }
                    _ => (),
                }
            }
        }
        for (_, body) in (&player, &mut bodies).join() {
            let body = body.get_mut(&mut physic_world);
            body.set_ang_acc_scale(::na::zero());
            body.set_ang_vel(::na::zero());

            let mut pos = body.position().clone();
            pos = ::na::Isometry3::new(
                ::na::Vector3::new(pos.translation.vector[0], pos.translation.vector[1], 0.5),
                ::na::Vector3::x() * ::std::f32::consts::FRAC_PI_2,
            );
            body.set_transformation(pos);
        }
    }
}

pub struct DrawSystem;

impl<'a> ::specs::System<'a> for DrawSystem {
    type SystemData = (::specs::ReadStorage<'a, ::component::StaticDraw>,
     ::specs::ReadStorage<'a, ::component::DynamicDraw>,
     ::specs::ReadStorage<'a, ::component::DynamicEraser>,
     ::specs::ReadStorage<'a, ::component::PhysicBody>,
     ::specs::ReadStorage<'a, ::component::Player>,
     ::specs::ReadStorage<'a, ::component::Aim>,
     ::specs::FetchMut<'a, ::resource::Rendering>,
     ::specs::FetchMut<'a, ::resource::ImGui>,
     ::specs::FetchMut<'a, ::resource::Graphics>,
     ::specs::Fetch<'a, ::resource::Config>,
     ::specs::Fetch<'a, ::resource::PhysicWorld>);

    fn run(&mut self, (static_draws, dynamic_draws, dynamic_erasers, bodies, players, aims, mut rendering, mut imgui, mut graphics, config, physic_world): Self::SystemData) {
        let mut future = Vec::new();

        // Compute view uniform
        let view_uniform_buffer_subbuffer = {
            let (_, player_aim, player_body) = (&players, &aims, &bodies).join().next().unwrap();

            let player_pos = player_body.get(&physic_world).position().clone();

            // IDEA: if we change -player.x here to + then it is fun
            let camera_top = if player_aim.dir[2].abs() > 0.8 {
                ::na::Rotation3::new(::na::Vector3::new(0.0, 0.0, -player_aim.x_dir)) *
                    ::na::Vector3::x() * -player_aim.dir[2].signum()
            } else {
                ::na::Vector3::z()
            };

            let view_matrix = {
                let i: ::na::Transform3<f32> =
                    ::na::Similarity3::look_at_rh(
                        &::na::Point3::from_coordinates(::na::Vector3::from(player_pos.translation.vector)),
                        &::na::Point3::from_coordinates(::na::Vector3::from(player_pos.translation.vector) + player_aim.dir),
                        &camera_top.into(),
                        // &::na::Point3::from_coordinates(::na::Vector3::from(pos.translation.vector) + ::na::Vector3::new(0.0, 0.0, -10.0)),
                        // &::na::Point3::from_coordinates(::na::Vector3::from(pos.translation.vector)),
                        // &[-1.0, 0.0, 0.0].into(),
                        1.0,
                        ).to_superset();
                i.unwrap()
            };

            let proj_matrix = ::na::Perspective3::new(
                graphics.width as f32 / graphics.height as f32,
                ::std::f32::consts::FRAC_PI_3,
                0.01,
                100.0,
            ).unwrap();

            let view_uniform = ::graphics::shader::draw1_vs::ty::View {
                view: view_matrix.into(),
                proj: proj_matrix.into(),
            };

            graphics.view_uniform_buffer.next(view_uniform).unwrap()
        };

        // Compute view set
        let view_set = Arc::new(
            graphics
                .draw1_view_descriptor_set_pool
                .next()
                .add_buffer(view_uniform_buffer_subbuffer)
                .unwrap()
                .build()
                .unwrap(),
        );

        // Compute command
        let mut command_buffer_builder =
            AutoCommandBufferBuilder::new(graphics.device.clone(), graphics.queue.family())
                .unwrap()
                .begin_render_pass(
                    graphics.framebuffer.clone(),
                    false,
                    vec![0u32.into(), 0u32.into(), 1f32.into()],
                )
                .unwrap();

        for static_draw in static_draws.join() {
            command_buffer_builder = command_buffer_builder
                .draw(
                    graphics.draw1_pipeline.clone(),
                    DynamicState::none(),
                    graphics.primitives_vertex_buffers[static_draw.primitive].clone(),
                    (view_set.clone(), static_draw.set.clone()),
                    ::graphics::shader::draw1_fs::ty::Group {
                        group_hb: high_byte(static_draw.group as u32),
                        group_lb: low_byte(static_draw.group as u32),
                        color: static_draw.color as u32,
                    },
                )
                .unwrap();
        }

        for dynamic_draw in dynamic_draws.join() {
            let world_trans_subbuffer = graphics
                .world_uniform_buffer
                .next(dynamic_draw.world_trans)
                .unwrap();

            let dynamic_draw_set = Arc::new(
                graphics
                    .draw1_dynamic_descriptor_set_pool
                    .next()
                    .add_buffer(world_trans_subbuffer)
                    .unwrap()
                    .build()
                    .unwrap(),
            );

            for &primitive in &dynamic_draw.primitives {
                command_buffer_builder = command_buffer_builder
                    .draw(
                        graphics.draw1_pipeline.clone(),
                        DynamicState::none(),
                        graphics.primitives_vertex_buffers[primitive.0].clone(),
                        (view_set.clone(), dynamic_draw_set.clone()),
                        ::graphics::shader::draw1_fs::ty::Group {
                            group_hb: high_byte(primitive.1 as u32),
                            group_lb: low_byte(primitive.1 as u32),
                            color: dynamic_draw.color as u32,
                        },
                    )
                    .unwrap();
            }
        }

        command_buffer_builder = command_buffer_builder.next_subpass(false).unwrap();

        for dynamic_eraser in dynamic_erasers.join() {
            let world_trans_subbuffer = graphics
                .world_uniform_buffer
                .next(dynamic_eraser.world_trans)
                .unwrap();

            let dynamic_draw_set = Arc::new(
                graphics
                    .draw1_dynamic_descriptor_set_pool
                    .next()
                    .add_buffer(world_trans_subbuffer)
                    .unwrap()
                    .build()
                    .unwrap(),
            );

            command_buffer_builder = command_buffer_builder
                .draw(
                    graphics.draw1_eraser_pipeline.clone(),
                    DynamicState::none(),
                    graphics.primitives_vertex_buffers[dynamic_eraser.primitive].clone(),
                    (view_set.clone(), dynamic_draw_set.clone()),
                    (),
                )
                .unwrap();
        }

        command_buffer_builder = command_buffer_builder
            .end_render_pass()
            .unwrap()
            .fill_buffer(graphics.tmp_erased_buffer.clone(), 0u32)
            .unwrap()
            .dispatch([graphics.width/64, graphics.height/64, 1], graphics.eraser1_pipeline.clone(), graphics.eraser1_descriptor_set.clone(), ())
            .unwrap()
            // TODO: make velocity it configurable
            .dispatch([(::graphics::GROUP_COUNTER_SIZE/64) as u32, 1, 1], graphics.eraser2_pipeline.clone(), graphics.eraser2_descriptor_set.clone(), 6.0*config.dt())
            .unwrap();

        rendering.command_buffer = Some(command_buffer_builder.build().unwrap());

        // Compute second command
        let second_command_buffer_builder = AutoCommandBufferBuilder::new(graphics.device.clone(), graphics.queue.family()).unwrap()
            .begin_render_pass(graphics.second_framebuffers[rendering.image_num.take().unwrap()].clone(), false, vec!())
            .unwrap()
            .draw(
                graphics.draw2_pipeline.clone(),
                DynamicState::none(),
                graphics.fullscreen_vertex_buffer.clone(),
                graphics.draw2_descriptor_set.clone(),
                ()
            )
            .unwrap()
            .draw(
                graphics.cursor_pipeline.clone(),
                DynamicState::none(),
                graphics.cursor_vertex_buffer.clone(),
                graphics.cursor_descriptor_set.clone(),
                ()
            )
            .unwrap();

        // Build imgui
        let ui = imgui.frame(
            rendering.size_points.take().unwrap(),
            rendering.size_pixels.take().unwrap(),
            config.dt().clone(),
        );
        ui.window(im_str!("Hello world"))
            .size((300.0, 100.0), ::imgui::ImGuiSetCond_FirstUseEver)
            .build(|| {
                ui.text(im_str!("Hello world!"));
                ui.separator();
                ui.text(im_str!("This...is...imgui-rs!"));
            });

        // TODO: change imgui so that it use an iterator instead of a callback
        let ref_cell_cmd_builder = RefCell::new(Some(second_command_buffer_builder));
        ui.render::<_, ()>(|ui, drawlist| {
            let mut cmd_builder = ref_cell_cmd_builder.borrow_mut().take().unwrap();
            // TODO: impl vertex for imgui in imgui
            let (vertex_buffer, vertex_buf_future) = ImmutableBuffer::from_iter(
                drawlist.vtx_buffer.iter().map(|vtx| {
                    ::graphics::SecondVertexImgui::from(vtx.clone())
                }),
                BufferUsage::vertex_buffer(),
                graphics.queue.clone(),
            ).unwrap();
            future.push(vertex_buf_future);

            let (index_buffer, index_buf_future) = ImmutableBuffer::from_iter(
                drawlist.idx_buffer.iter().cloned(),
                BufferUsage::index_buffer(),
                graphics.queue.clone(),
            ).unwrap();
            future.push(index_buf_future);

            let (width, height) = ui.imgui().display_size();
            let (scale_width, scale_height) = ui.imgui().display_framebuffer_scale();

            let matrix = [
                [2.0 / width as f32, 0.0, 0.0, 0.0],
                [0.0, 2.0 / -(height as f32), 0.0, 0.0],
                [0.0, 0.0, -1.0, 0.0],
                [-1.0, 1.0, 0.0, 1.0],
            ];

            let (matrix, matrix_future) = ImmutableBuffer::from_data(
                matrix,
                BufferUsage::uniform_buffer(),
                graphics.queue.clone(),
            ).unwrap();
            future.push(matrix_future);

            let matrix_set = Arc::new(
                graphics
                    .imgui_matrix_descriptor_set_pool
                    .next()
                    .add_buffer(matrix)
                    .unwrap()
                    .build()
                    .unwrap(),
            );

            for cmd in drawlist.cmd_buffer {
                let dynamic_state = DynamicState {
                    line_width: None,
                    viewports: None,
                    scissors: Some(vec![
                        Scissor {
                            origin: [
                                (cmd.clip_rect.x * scale_width) as i32,
                                ((height - cmd.clip_rect.w) * scale_height) as i32,
                            ],
                            dimensions: [
                                ((cmd.clip_rect.z - cmd.clip_rect.x) * scale_width) as u32,
                                ((cmd.clip_rect.w - cmd.clip_rect.y) * scale_height) as u32,
                            ],
                        },
                    ]),
                };

                cmd_builder = cmd_builder
                    .draw_indexed(
                        graphics.imgui_pipeline.clone(),
                        dynamic_state.clone(),
                        vertex_buffer.clone(), index_buffer.clone(),
                        (matrix_set.clone(), graphics.imgui_descriptor_set.clone()),
                        ()
                    )
                    .unwrap();
            }
            *ref_cell_cmd_builder.borrow_mut() = Some(cmd_builder);
            Ok(())
        }).unwrap();

        let second_command_buffer_builder = ref_cell_cmd_builder.borrow_mut().take().unwrap();

        rendering.second_command_buffer = Some(
            second_command_buffer_builder
                .end_render_pass()
                .unwrap()
                .build()
                .unwrap(),
        );
    }
}

pub struct UpdateDynamicDrawEraserSystem;

impl<'a> ::specs::System<'a> for UpdateDynamicDrawEraserSystem {
    type SystemData = (::specs::ReadStorage<'a, ::component::PhysicBody>,
     ::specs::WriteStorage<'a, ::component::DynamicDraw>,
     ::specs::WriteStorage<'a, ::component::DynamicEraser>,
     ::specs::Fetch<'a, ::resource::PhysicWorld>);

    fn run(
        &mut self,
        (bodies, mut dynamic_draws, mut dynamic_erasers, physic_world): Self::SystemData,
    ) {
        for (dynamic_draw, body) in (&mut dynamic_draws, &bodies).join() {
            let trans = body.get(&physic_world).position() * dynamic_draw.primitive_trans;
            dynamic_draw.world_trans =
                ::graphics::shader::draw1_vs::ty::World { world: trans.unwrap().into() }
        }

        for (dynamic_eraser, body) in (&mut dynamic_erasers, &bodies).join() {
            let trans = body.get(&physic_world).position() * dynamic_eraser.primitive_trans;
            dynamic_eraser.world_trans =
                ::graphics::shader::draw1_vs::ty::World { world: trans.unwrap().into() }
        }
    }
}

pub struct ShootSystem;

// TODO: not shoot yourself and shoot in one direction only
impl<'a> ::specs::System<'a> for ShootSystem {
    type SystemData = (::specs::ReadStorage<'a, ::component::PhysicBody>,
     ::specs::ReadStorage<'a, ::component::Aim>,
     ::specs::WriteStorage<'a, ::component::Shooter>,
     ::specs::WriteStorage<'a, ::component::Life>,
     ::specs::Fetch<'a, ::resource::PhysicWorld>,
     ::specs::Fetch<'a, ::resource::Config>);

    fn run(
        &mut self,
        (bodies, aims, mut shooters, mut _lifes, physic_world, config): Self::SystemData,
    ) {
        for (aim, body, shooter) in (&aims, &bodies, &mut shooters).join() {
            let body_pos = body.get(&physic_world).position().clone();
            shooter.reload(config.dt().clone());

            let _ray = ::ncollide::query::Ray {
                origin: ::na::Point3::from_coordinates(body_pos.translation.vector),
                dir: aim.dir,
            };

            let _group = ::ncollide::world::CollisionGroups::new();

            if shooter.do_shoot() {
                // for (entity, _body, _collision) in physic_world.collision_world().interferences_with_ray(&ray, &group) {
                //     if let Some(ref mut life) = lifes.get_mut(entity) {
                //         life.0 -= 1;
                //     }
                // }
            }
        }
    }
}
