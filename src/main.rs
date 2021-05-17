use bevy::prelude::*;
use pathfinding::prelude::*;
use std::ops::Not;

const SIZE: i32 = 10;

fn main() {
    let mut app = App::build();
    app.insert_resource(ClearColor(Color::GRAY))
        .insert_resource(WindowDescriptor {
            title: "Pathfinding".to_string(),
            width: 800.,
            height: 600.,
            vsync: false,
            resizable: false,
            ..Default::default()
        })
        .add_event::<ToggleBlockEvent>()
        .init_resource::<Materials>()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system_to_stage(CoreStage::PostUpdate, grid_to_transform.system())
        .add_system(mouse_click_system.system())
        .add_system(toggle_block.system())
        .add_system(pathfinding.system());
    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);
    app.run();
}

struct MainCamera;
struct Start;
struct End;
struct Block;

#[derive(Default)]
struct Materials {
    path: Option<Handle<ColorMaterial>>,
    block: Option<Handle<ColorMaterial>>,
}

#[derive(Eq, PartialEq, Copy, Clone, Hash, Debug)]
struct Pos {
    x: i32,
    y: i32,
}
impl Pos {
    fn try_new(x: i32, y: i32) -> Option<Self> {
        if x < 0 || y < 0 || x >= SIZE as i32 || y >= SIZE as i32 {
            None
        } else {
            Some(Self {
                x: x as i32,
                y: y as i32,
            })
        }
    }

    fn min(&self) -> bool {
        self.x == 0 && self.y == 0
    }

    fn max(&self) -> bool {
        self.x == SIZE - 1 && self.y == SIZE - 1
    }
}

struct ToggleBlockEvent {
    pos: Pos,
}

struct Path;

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut my_materials: ResMut<Materials>,
) {
    my_materials.path = Some(materials.add(Color::rgb(1., 1., 1.).into()));
    my_materials.block = Some(materials.add(Color::rgb(0.5, 0.5, 1.0).into()));

    commands
        .spawn_bundle(OrthographicCameraBundle::new_2d())
        .insert(MainCamera);
    commands.spawn_bundle(UiCameraBundle::default());

    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite::new(Vec2::new(35., 35.)),
            material: materials.add(Color::rgb(1., 1., 1.).into()),
            ..Default::default()
        })
        .insert(Pos::try_new(0, 0).unwrap())
        .insert(Start);

    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite::new(Vec2::new(35., 35.)),
            material: materials.add(Color::rgb(1., 1., 1.).into()),
            ..Default::default()
        })
        .insert(Pos::try_new(9, 9).unwrap())
        .insert(End);

    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite::new(Vec2::new(400., 400.)),
        material: materials.add(Color::rgb(0., 0., 0.).into()),
        transform: Transform::from_xyz(-20., -20., 1.),
        ..Default::default()
    });
}

fn grid_to_transform(query: Query<(&Pos, &mut Transform)>) {
    query.for_each_mut(|(pos, mut transform): (&Pos, Mut<Transform>)| {
        transform.translation.x = ((pos.x as i32 * 40) - 200) as f32;
        transform.translation.y = ((pos.y as i32 * 40) - 200) as f32;
        transform.translation.z = 2.;
    });
}

fn mouse_click_system(
    mouse_button_input: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    mut my_events: EventWriter<ToggleBlockEvent>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        if let Some(window) = windows.get_primary() {
            if let Some(cursor_pos) = window.cursor_position() {
                let x = (cursor_pos.x as i32 - 180) / 40;
                let y = (cursor_pos.y as i32 - 85) / 40;

                if let Some(pos) = Pos::try_new(x as i32, y as i32) {
                    my_events.send(ToggleBlockEvent { pos });
                }
            }
        }
    }
}

fn toggle_block(
    mut my_events: EventReader<ToggleBlockEvent>,
    blocks: Query<(Entity, &Pos), With<Block>>,
    mut commands: Commands,
    mut materials: Res<Materials>,
) {
    for event in my_events.iter() {
        let event: &ToggleBlockEvent = event;
        if event.pos.min() || event.pos.max() {
            continue;
        }
        match blocks.iter().find(|(_, pos)| pos == &&event.pos) {
            None => {
                commands
                    .spawn_bundle(SpriteBundle {
                        sprite: Sprite::new(Vec2::new(35., 35.)),
                        material: materials.block.clone().unwrap(),
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

/// Pathfinding logic
/// find shortest path between Start and End
fn pathfinding(
    start: Query<&Pos, With<Start>>,
    end: Query<&Pos, With<End>>,
    blocks: Query<&Pos, With<Block>>,
    paths: Query<Entity, With<Path>>,
    mut commands: Commands,
    materials: Res<Materials>,
) {
    let start = start.single().expect("No start block");
    let end = end.single().expect("No end block");

    let blocks = blocks.iter().collect::<Vec<_>>();

    let result = bfs(
        start,
        |p| {
            let &Pos { x, y } = p;
            vec![(x, y - 1), (x, y + 1), (x - 1, y), (x + 1, y)]
                .into_iter()
                .filter_map(|(x, y)| Pos::try_new(x, y))
                .filter(|pos| blocks.contains(&pos).not())
        },
        |p| p == end,
    );

    for entity in paths.iter() {
        commands.entity(entity).despawn_recursive();
    }

    if let Some(path) = result {
        for pos in path {
            commands
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite::new(Vec2::new(5., 5.)),
                    material: materials.path.clone().unwrap(),
                    ..Default::default()
                })
                .insert(pos)
                .insert(Path);
        }
    }
}
