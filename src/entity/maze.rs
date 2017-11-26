use std::collections::HashMap;
use std::f32::consts::FRAC_PI_2;

pub fn create_maze_walls_w(
    colors: &HashMap<::na::Vector2<isize>, ::graphics::Color>,
    world: &mut ::specs::World,
) {
    create_maze_walls(
        &colors,
        &mut world.write(),
        &mut world.write(),
        &mut world.write_resource(),
        &world.read_resource(),
        &world.read_resource(),
        &world.read_resource(),
        &world.read_resource(),
    );
}

pub fn create_maze_walls<'a>(
    colors: &HashMap<::na::Vector2<isize>, ::graphics::Color>,
    bodies: &mut ::specs::WriteStorage<'a, ::component::PhysicBody>,
    static_draws: &mut ::specs::WriteStorage<'a, ::component::StaticDraw>,
    physic_world: &mut ::specs::FetchMut<'a, ::resource::PhysicWorld>,
    graphics: &::specs::Fetch<'a, ::resource::Graphics>,
    maze: &::specs::Fetch<'a, ::resource::Maze>,
    config: &::specs::Fetch<'a, ::resource::Config>,
    entities: &::specs::Entities,
) {
    super::create_floor_ceil(
        0.0,
        true,
        bodies,
        static_draws,
        physic_world,
        graphics,
        config,
        entities,
    );
    super::create_floor_ceil(
        1.0,
        false,
        bodies,
        static_draws,
        physic_world,
        graphics,
        config,
        entities,
    );

    let mut create_wall_side_closure = |pos, x_radius, y_radius, color| {
        super::create_wall_side(
            pos,
            x_radius,
            y_radius,
            color,
            bodies,
            static_draws,
            physic_world,
            graphics,
            config,
            entities,
        );
    };

    let minus_x_sides = maze.compute_zones(|maze, cell| {
        let open = cell + ::na::Vector2::new(-1, 0);
        maze.walls.contains(cell) && !maze.walls.contains(&open) && !colors.contains_key(&open)
    });
    let plus_x_sides = maze.compute_zones(|maze, cell| {
        let open = cell + ::na::Vector2::new(1, 0);
        maze.walls.contains(cell) && !maze.walls.contains(&open) && !colors.contains_key(&open)
    });
    let minus_y_sides = maze.compute_zones(|maze, cell| {
        let open = cell + ::na::Vector2::new(0, -1);
        maze.walls.contains(cell) && !maze.walls.contains(&open) && !colors.contains_key(&open)
    });
    let plus_y_sides = maze.compute_zones(|maze, cell| {
        let open = cell + ::na::Vector2::new(0, 1);
        maze.walls.contains(cell) && !maze.walls.contains(&open) && !colors.contains_key(&open)
    });

    for (dx, x_side) in minus_x_sides
        .iter()
        .map(|side| (::na::Vector3::new(-0.5, 0.0, 0.0), side))
        .chain(plus_x_sides.iter().map(|side| {
            (::na::Vector3::new(0.5, 0.0, 0.0), side)
        }))
    {
        let x = x_side.iter().next().unwrap()[0];
        let (y_min, y_max) = x_side.iter().fold(
            (isize::max_value(), isize::min_value()),
            |acc, cell| {
                (acc.0.min(cell[1]), acc.1.max(cell[1]))
            },
        );
        let x_radius = 0.5;
        let y_radius = (y_max - y_min + 1) as f32 / 2.0;
        let pos = ::na::Isometry3::new(
            ::na::Vector3::new(x as f32 + 0.5, y_min as f32 + y_radius, 0.5) + dx,
            ::na::Vector3::y() * FRAC_PI_2,
        );
        create_wall_side_closure(pos, x_radius, y_radius, None);
    }

    for (dy, y_side) in minus_y_sides
        .iter()
        .map(|side| (::na::Vector3::new(0.0, -0.5, 0.0), side))
        .chain(plus_y_sides.iter().map(|side| {
            (::na::Vector3::new(0.0, 0.5, 0.0), side)
        }))
    {
        let y = y_side.iter().next().unwrap()[1];
        let (x_min, x_max) = y_side.iter().fold(
            (isize::max_value(), isize::min_value()),
            |acc, cell| {
                (acc.0.min(cell[0]), acc.1.max(cell[0]))
            },
        );
        let y_radius = 0.5;
        let x_radius = (x_max - x_min + 1) as f32 / 2.0;
        let pos = ::na::Isometry3::new(
            ::na::Vector3::new(x_min as f32 + x_radius, y as f32 + 0.5, 0.5) + dy,
            ::na::Vector3::x() * FRAC_PI_2,
        );
        create_wall_side_closure(pos, x_radius, y_radius, None);
    }

    for (pos, &color) in colors {
        if maze.walls.contains(&(pos + ::na::Vector2::new(-1, 0))) {
            let i = ::na::Isometry3::new(
                ::na::Vector3::new(pos[0] as f32, pos[1] as f32 + 0.5, 0.5),
                ::na::Vector3::y() * FRAC_PI_2,
            );
            create_wall_side_closure(i, 0.5, 0.5, Some(color));
        }
        if maze.walls.contains(&(pos + ::na::Vector2::new(1, 0))) {
            let i = ::na::Isometry3::new(
                ::na::Vector3::new(pos[0] as f32 + 1.0, pos[1] as f32 + 0.5, 0.5),
                ::na::Vector3::y() * FRAC_PI_2,
            );
            create_wall_side_closure(i, 0.5, 0.5, Some(color));
        }
        if maze.walls.contains(&(pos + ::na::Vector2::new(0, -1))) {
            let i = ::na::Isometry3::new(
                ::na::Vector3::new(pos[0] as f32 + 0.5, pos[1] as f32, 0.5),
                ::na::Vector3::x() * FRAC_PI_2,
            );
            create_wall_side_closure(i, 0.5, 0.5, Some(color));
        }
        if maze.walls.contains(&(pos + ::na::Vector2::new(0, 1))) {
            let i = ::na::Isometry3::new(
                ::na::Vector3::new(pos[0] as f32 + 0.5, pos[1] as f32 + 1.0, 0.5),
                ::na::Vector3::x() * FRAC_PI_2,
            );
            create_wall_side_closure(i, 0.5, 0.5, Some(color));
        }
    }
}
