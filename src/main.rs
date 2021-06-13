use bevy::prelude::*;
use rand::random;
use bevy::core::FixedTimestep;

const WIDTH: u32 = 30;
const HEIGHT: u32 = 30;
const SPAWN_RATE: f64 = 0.5;
const MOVEMENT_RATE: f64 = 5.0;

#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
struct Position {
    x: i32,
    y: i32,
}

struct Size {
    width: f32,
    height: f32,
}

impl Size {
    pub fn square(x: f32) -> Self {
        Self {
            height: x,
            width: x,
        }
    }
}

fn main() {
    let movement: f64 = 1.0 / MOVEMENT_RATE;
    let fruits: f64 = 1.0 / SPAWN_RATE;

    App::build()
        .insert_resource(WindowDescriptor {
            title: "Snek".to_string(),
            width: 1000.0,
            height: 1000.0,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
        .insert_resource(SnekSegments::default())
        .insert_resource(LastTailPosition::default())
        .add_startup_system(setup.system())
        // we need a new stage here, since the material used here is created in the setup system.
        // "single" means that there's only one system called in this stage. there are other options like "serial" or "parallel"
        .add_startup_stage("game_setup", SystemStage::single(spawn_snek.system()))
        .add_system(
            snek_movement_input.system()
                .label(SnekMovement::Input)
                .before(SnekMovement::Movement) // we make sure that we get the input before moving the snek
        )
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(movement))
                .with_system(snek_movement.system().label(SnekMovement::Movement)) // we label this system "movement"
                .with_system(
                    snek_eating.system()
                        .label(SnekMovement::Eating)
                        .after(SnekMovement::Movement)
                )
                .with_system(
                    snek_growth.system()
                        .label(SnekMovement::Growth)
                        .after(SnekMovement::Eating)
                )
        )
        .add_system(game_over.system().after(SnekMovement::Movement))
        .add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new()
                .with_system(position_translation.system())
                .with_system(size_scaling.system()),
        )
        .add_system_set(
            SystemSet::new()
                // food should only spawn every second.
                .with_run_criteria(FixedTimestep::step(fruits))
                .with_system(food_spawner.system())
        )
        .add_event::<GrowthEvent>()
        .add_event::<GameOverEvent>()
        .add_plugins(DefaultPlugins)
        .run();
}

fn setup(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    // A new 2d camera is created.
    // We can use a camera bundle for this, which spawns a new camera entity
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.insert_resource(Materials {
        // create a new materials struct. add() - method returns a handle like it is defined in the mat. struct.
        head_material: materials.add(Color::hex("EFEFEF").unwrap().into()),
        food_material: materials.add(Color::hex("ABFF8B").unwrap().into()),
        segment_material: materials.add(Color::hex("B2B2B2").unwrap().into()),
    })
}

// this system looks for a resource of struct "Materials" which we created
fn spawn_snek(mut commands: Commands, materials: Res<Materials>, mut segments: ResMut<SnekSegments>) {
    // we spawn a sprite
    println!("{:?}", segments.0);
    segments.0 = vec![
        commands.spawn_bundle(SpriteBundle {
            material: materials.head_material.clone(), //material is the head_material which we added to the resources
            sprite: Sprite::new(Vec2::new(10.0, 10.0)), // create a new sprite - 2 dimensional with size 10, 10
            ..Default::default() // other attributes are default
        })
            .insert(SnekHead {
                direction: Direction::Up,
                next_direction: Direction::Up
            }) // we insert SnekHead as a component into this new snek-entity
            .insert(Position {
                x: 3,
                y: 3,
            })
            .insert(Size::square(0.8))
            .insert(SnekSegment)
            .id(),
        spawn_segment(commands,
                      &materials.segment_material,
                      Position {
                          x: 3,
                          y: 2,
                      }),
    ];
}

fn size_scaling(windows: Res<Windows>, mut query: Query<(&Size, &mut Sprite)>) {
    let window = windows.get_primary().unwrap();
    for (sprite_size, mut sprite) in query.iter_mut() {
        sprite.size = Vec2::new(
            sprite_size.width / WIDTH as f32 * window.width() as f32,
            sprite_size.height / HEIGHT as f32 * window.height() as f32,
        )
    }
}

fn position_translation(windows: Res<Windows>, mut q: Query<(&Position, &mut Transform)>) {
    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window  // translate the tile position to pixel position
            - (bound_window / 2.0)  // coordinate 0:0 is right in the middle of the screen. so we subtract half the screen
            + (tile_size / 2.0) // then add half a tile because the tile also has 0:0 in the center
    }

    let window = windows.get_primary().unwrap();
    for (pos, mut transform) in q.iter_mut() {
        transform.translation = Vec3::new(
            convert(pos.x as f32, window.width(), WIDTH as f32),
            convert(pos.y as f32, window.height(), HEIGHT as f32),
            0.0,
        )
    }
}

fn snek_movement(segments: ResMut<SnekSegments>,
                 mut heads: Query<(Entity, &mut SnekHead)>,
                 mut positions: Query<&mut Position>,
                 mut last_tail_position: ResMut<LastTailPosition>,
                 mut game_over_writer: EventWriter<GameOverEvent>,
) {
    if let Some((head_entity, mut head)) = heads.iter_mut().next() {
        let segment_positions = segments.0.iter()
            .map(|e| *positions.get_mut(*e).unwrap())
            .collect::<Vec<Position>>();

        let mut head_pos = positions.get_mut(head_entity).unwrap();
        head.direction = head.next_direction;
        match &head.direction {
            Direction::Left => head_pos.x -= 1,
            Direction::Right => head_pos.x += 1,
            Direction::Up => head_pos.y += 1,
            Direction::Down => head_pos.y -= 1,
        }
        if head_pos.x < 0
            || head_pos.x as u32 >= WIDTH
            || head_pos.y < 0
            || head_pos.y as u32 >= HEIGHT {
            game_over_writer.send(GameOverEvent);
        }

        if segment_positions.contains(&head_pos) {
            game_over_writer.send(GameOverEvent);
        }

        segment_positions.iter()
            .zip(segments.0.iter().skip(1))
            .for_each(|(segpos, segment)| {
                *positions.get_mut(*segment).unwrap() = *segpos;
            });
        last_tail_position.0 = Some(*segment_positions.last().unwrap());
    }
}

fn snek_movement_input(keyboard_input: Res<Input<KeyCode>>, mut heads: Query<&mut SnekHead>) {
    if let Some(mut head) = heads.iter_mut().next() {
        let direction: Direction = if keyboard_input.pressed(KeyCode::A) {
            Direction::Left
        } else if keyboard_input.pressed(KeyCode::D) {
            Direction::Right
        } else if keyboard_input.pressed(KeyCode::W) {
            Direction::Up
        } else if keyboard_input.pressed(KeyCode::S) {
            Direction::Down
        } else {
            head.direction
        };

        if direction != head.direction.opposite() && direction != head.direction {
            head.next_direction = direction;
        }
    }
}

fn food_spawner(mut commands: Commands, materials: Res<Materials>, positions: Query<&Position, With<SnekSegment>>) {
    let gen = || {
        Position{
            x: (random::<f32>() * WIDTH as f32) as i32,
            y: (random::<f32>() * HEIGHT as f32) as i32,
        }
    };

    let positions_vec: Vec<&Position> = positions.iter().collect();
    let mut pos = gen();
    while positions_vec.contains(&&pos) {
        pos = gen();
    }


    commands.spawn_bundle(SpriteBundle {
        material: materials.food_material.clone(),
        ..Default::default()
    })
        .insert(Food)
        .insert(pos)
        .insert(Size::square(0.7));
}

fn spawn_segment(mut commands: Commands, material: &Handle<ColorMaterial>, position: Position) -> Entity {
    commands.spawn_bundle(SpriteBundle {
        material: material.clone(),
        ..Default::default()
    })
        .insert(SnekSegment)
        .insert(position)
        .insert(Size::square(0.5))
        .id()
}

fn snek_eating(mut commands: Commands,
               mut growth_writer: EventWriter<GrowthEvent>,
               food_positions: Query<(Entity, &Position), With<Food>>,
               head_positions: Query<&Position, With<SnekHead>>,
) {
    for head_pos in head_positions.iter() {
        for (ent, food_pos) in food_positions.iter() {
            if food_pos == head_pos {
                commands.entity(ent).despawn();
                growth_writer.send(GrowthEvent);
            }
        }
    }
}

fn snek_growth(
    commands: Commands,
    last_tail_position: Res<LastTailPosition>,
    mut segments: ResMut<SnekSegments>,
    mut growth_reader: EventReader<GrowthEvent>,
    materials: Res<Materials>,
) {
    if growth_reader.iter().next().is_some() {
        segments.0.push(spawn_segment(
            commands,
            &materials.segment_material,
            last_tail_position.0.unwrap(),
        ))
    }
}

fn game_over(
    mut game_over_reader: EventReader<GameOverEvent>,
    mut commands: Commands,
    materials: Res<Materials>,
    segments_res: ResMut<SnekSegments>,
    food: Query<Entity, With<Food>>,
    segments: Query<Entity, With<SnekSegment>>,
) {
    if game_over_reader.iter().next().is_some() {
        for ent in food.iter().chain(segments.iter()) {
            commands.entity(ent).despawn();
        }

        spawn_snek(commands, materials, segments_res);
    }
}

struct SnekHead {
    direction: Direction,
    next_direction: Direction
}

struct Food;

// This struct is used like a tag, so we can query for it later.
struct Materials {
    // This struct will be a resource which stores materials for various components.
    head_material: Handle<ColorMaterial>,
    food_material: Handle<ColorMaterial>,
    segment_material: Handle<ColorMaterial>,
}

#[derive(PartialEq, Copy, Clone, Debug)]
enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }
}

#[derive(SystemLabel, Debug, Hash, PartialEq, Eq, Clone)]
enum SnekMovement {
    Input,
    Movement,
    Eating,
    Growth,
}

struct SnekSegment;

#[derive(Default)]
struct SnekSegments(Vec<Entity>);

struct GrowthEvent;

#[derive(Default)]
struct LastTailPosition(Option<Position>);

struct GameOverEvent;

