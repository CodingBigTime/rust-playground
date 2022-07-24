use bevy::DefaultPlugins;
use bevy::prelude::*;

#[derive(Default)]
struct Mouse {
    position: Vec2,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.25, 0.75),
                custom_size: Some(Vec2::new(16.0, 16.0)),
                ..default()
            },
            ..default()
        })
        .add_startup_system(setup)
        // .add_startup_system(|mut windows: ResMut<Windows>| windows.get_primary_mut().unwrap().set_cursor_visibility(false))
        .add_system(move_mouse)
        .insert_resource(Mouse { position: Vec2::default() })
        .run();
}

fn move_mouse(
    mut event: EventReader<CursorMoved>,
    mut mouse: ResMut<Mouse>,
    mut query: Query<&mut GlobalTransform, With<Sprite>>,
    camera: Query<&Transform, With<Camera>>,
    windows: ResMut<Windows>,
) {
    if let Some(cursor_moved) = event.iter().last() {
        mouse.position = cursor_moved.position;
    }
    for mut transform in query.iter_mut() {
        transform.translation = window_to_world(mouse.position, windows.primary(), camera.iter().next().unwrap());
    }
}

fn setup(mut commands: Commands, sprite_bundle: Res<SpriteBundle>) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(sprite_bundle.clone());
}

fn window_to_world(
    position: Vec2,
    window: &Window,
    camera: &Transform,
) -> Vec3 {

    // Center in screen space
    let norm = Vec3::new(
        position.x - window.width() / 2.,
        position.y - window.height() / 2.,
        0.,
    );

    // Apply camera transform
    *camera * norm

    // Alternatively:
    //camera.mul_vec3(norm)
}

