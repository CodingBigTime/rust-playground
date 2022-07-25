use bevy::{prelude::*, DefaultPlugins, window::PresentMode};

#[derive(Component)]
struct MouseSquare;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(WindowDescriptor {
            title: "Coding Big Time - Blue Square".to_string(),
            width: 800.,
            height: 600.,
            present_mode: PresentMode::Immediate,
            ..Default::default()
        })
        .add_startup_system(setup)
        // .add_startup_system(|mut windows: ResMut<Windows>| windows.get_primary_mut().unwrap().set_cursor_visibility(false))
        .add_system(move_mouse)
        .run();
}

fn move_mouse(
    windows: Res<Windows>,
    camera_transform: Query<&Transform, (With<Camera>, Without<MouseSquare>)>,
    mut mouse_square_transform: Query<&mut Transform, (With<MouseSquare>, Without<Camera>)>,
) {
    let window = windows.get_primary().unwrap();
    if let Some(cursor_position) = window.cursor_position()  {
        mouse_square_transform.single_mut().translation = window_to_world(
            cursor_position, window, camera_transform.single(),
        );
    }
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            color: Color::rgb(0.25, 0.25, 0.75),
            custom_size: Some(Vec2::new(16.0, 16.0)),
            ..default()
        },
        ..default()
    }).insert(MouseSquare);
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

