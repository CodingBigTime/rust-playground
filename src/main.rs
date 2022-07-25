use bevy::{prelude::*, DefaultPlugins, window::PresentMode};

#[derive(Component)]
struct MouseSquare {
    index: usize
}

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
    mut mouse_square_query: Query<(&mut Transform, &MouseSquare), Without<Camera>>,
) {
    let window = windows.get_primary().unwrap();
    let mut mouse_squares = mouse_square_query.iter_mut().collect::<Vec<_>>();
    mouse_squares.sort_by(|a, b| a.1.index.cmp(&b.1.index));

    for i in 0..mouse_squares.len()-1 {
        mouse_squares[i].0.translation = mouse_squares[i+1].0.translation;
    }
    
    if let Some(cursor_position) = window.cursor_position()  {
        mouse_squares.last_mut().unwrap().0.translation = window_to_world(
            cursor_position, window, camera_transform.single(),
        );
    }
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    const NUM_SQUARES: usize = 20;
    for i in 0..NUM_SQUARES {
        commands.spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::hsl(i as f32 / NUM_SQUARES as f32 * 360., 0.5, 0.5),
                custom_size: Some(Vec2::new(16.0, 16.0)),
                ..default()
            },
            ..default()
        }).insert(MouseSquare {index: i});
    }
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

