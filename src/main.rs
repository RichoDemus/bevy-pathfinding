use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::*;
use bevy::render::camera::Camera;
use pathfinding::prelude::*;
use std::ops::Not;

const SIZE:u8 = 10;

fn main() {
    App::build()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(WindowDescriptor {
            title: "Pathfinding".to_string(),
            width: 800.,
            height: 600.,
            vsync: false,
            resizable: false,
            ..Default::default()
        })
        .add_event::<ToggleBlockEvent>()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system_to_stage(CoreStage::PostUpdate, grid_to_transform.system())
        .add_system(mouse_click_system.system())
        .add_system(toggle_block.system())
        .add_system(pathfinding.system())
        .run();
}

struct MainCamera;
struct Start;
struct End;
struct Block;

#[derive(Eq, PartialEq, Copy, Clone, Hash, Debug)]
struct Pos {
    x:u8,
    y:u8,
}
impl Pos {
    fn new(x:u8, y:u8) -> Self {
        debug_assert!(x < SIZE);
        debug_assert!(y < SIZE);
        Self {
            x,
            y,
        }
    }

    fn try_new(x: i8, y: i8) -> Option<Self> {
        if x < 0 || y < 0 || x >= SIZE as i8 || y >= SIZE as i8 {
            None
        } else {
            Some(Self {
                x: x as u8,
                y: y as u8,
            })
        }
    }

    fn min(&self) -> bool {
        self.x == 0 && self.y == 0
    }

    fn max(&self) -> bool {
        self.x == SIZE-1 && self.y == SIZE-1
    }
}

struct ToggleBlockEvent {
    pos: Pos,
}

struct Path;

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d()).insert(MainCamera);
    commands.spawn_bundle(UiCameraBundle::default());

            commands.spawn_bundle(SpriteBundle {
                sprite: Sprite::new(Vec2::new(35., 35.)),
                material: materials.add(Color::rgb(1., 1., 1.).into()),
                ..Default::default()
            })
                .insert(Pos::new(0,0))
                .insert(Start);

    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite::new(Vec2::new(35., 35.)),
        material: materials.add(Color::rgb(1., 1., 1.).into()),
        ..Default::default()
    })
        .insert(Pos::new(9,9))
        .insert(End);
}

fn grid_to_transform(
    mut query: Query<(&Pos, &mut Transform)>,
) {
    query.for_each_mut(|(pos, mut transform):(&Pos, Mut<Transform>)|{
        transform.translation.x = ((pos.x as i16 * 40) - 200) as f32;
        transform.translation.y = ((pos.y as i16 * 40) - 200) as f32;
    });
}

// bottom left: 200,100
// top right: 560, 465

fn mouse_click_system(
    mouse_button_input: Res<Input<MouseButton>>,
    camera_query: Query<(&GlobalTransform, &Transform, &Camera), With<MainCamera>>,
    windows: Res<Windows>,
    mut my_events: EventWriter<ToggleBlockEvent>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        if let Ok((global_transform, transform, camera)) = camera_query.single() {
            if let Some(window) = windows.get_primary() {
                if let Some(cursor_pos) = window.cursor_position() {
                    let x = (cursor_pos.x as i16 - 180) / 40;
                    let y = (cursor_pos.y as i16 - 85) / 40;
                    info!("cursor: {},{} grid: {},{}", cursor_pos.x, cursor_pos.y, x,y);
                    if let Some(pos) = Pos::try_new(x as i8,y as i8) {
                        my_events.send(ToggleBlockEvent {
                           pos
                        });
                    }
                }
            }
        }
    }
}

fn toggle_block(
    mut my_events: EventReader<ToggleBlockEvent>,
    blocks: Query<(Entity, &Pos), With<Block>>,
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for event in my_events.iter() {
        let event: &ToggleBlockEvent = event;
        if event.pos.min() || event.pos.max() {
            continue;
        }
        match blocks.iter().find(|(_, pos)| pos == &&event.pos) {
            None => {
                commands.spawn_bundle(SpriteBundle {
                    sprite: Sprite::new(Vec2::new(35., 35.)),
                    material: materials.add(Color::rgb(0.5, 0.5, 1.0).into()),
                    ..Default::default()
                })
                    .insert(event.pos)
                    .insert(Block);
            }
            Some((entity, _)) => {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

fn get_camera_position_in_world_coordinates(
    windows: &Res<Windows>,
    camera_query: &Query<&GlobalTransform, With<MainCamera>>,
) -> Option<Vec2> {
    if let Some(window) = windows.get_primary() {
        if let Some(cursor_position) = window.cursor_position() {
            if let Ok(global_transform) = camera_query.single() {
                let norm = Vec3::new(
                    cursor_position.x - window.width() / 2.,
                    cursor_position.y - window.height() / 2.,
                    0.,
                );

                let pos = *global_transform * norm;
                return Some(pos.truncate());
            }
        }
    }
    None
}

fn pathfinding(
    start: Query<&Pos, With<Start>>,
    end: Query<&Pos, With<End>>,
    blocks: Query<&Pos, With<Block>>,
    paths: Query<Entity, With<Path>>,
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let start = start.single().expect("No start block");
    let end = end.single().expect("No end block");

    let blocks = blocks.iter().collect::<Vec<_>>();

    let result = bfs(
        start,
        |p| {
            let x = p.x as i8;
            let y = p.y as i8;
            vec![
                (x, y - 1),
                (x, y + 1),
                (x - 1, y),
                (x + 1, y),
            ].into_iter()
                .filter_map(|(x,y)|Pos::try_new(x,y))
                .filter(|pos| {
                    blocks.contains(&pos).not()
                })
        },
        |p|p == end,
    );

    for (entity) in paths.iter() {
        commands.entity(entity).despawn_recursive();
    }

    if let Some(path) = result {
        for pos in path {
            commands.spawn_bundle(SpriteBundle {
                sprite: Sprite::new(Vec2::new(5., 5.)),
                material: materials.add(Color::rgb(1., 1., 1.).into()),
                ..Default::default()
            })
                .insert(pos)
                .insert(Path);
        }
    }
}
