use bevy::core_pipeline::bloom::BloomSettings;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::input::mouse::*;
use bevy::prelude::*;
use bevy_easings::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::prelude::*;
use rand::prelude::*;
use std::time::Duration;

#[derive(Bundle)]
struct PositionedParticle {
    rigid_body: RigidBody,
    easing: EasingComponent<Sprite>,
    collider: Collider,
    restitution: Restitution,
    velocity: Velocity,

    #[bundle]
    sprite: SpriteBundle,
}

impl PositionedParticle {
    fn new(x: f32, y: f32, size: f32) -> Self {
        let mut rng = rand::thread_rng();
        let angle = rng.gen_range(0.0..2. * std::f32::consts::PI);
        let dx = angle.sin() * 100.0;
        let dy = angle.cos() * 100.0;
        let mut l = [0.75, 0.0, 0.0];
        l.shuffle(&mut rng);
        Self {
            rigid_body: RigidBody::Dynamic,
            easing: Sprite {
                color: Color::rgb(l[0], l[1], l[2]),
                custom_size: Some(Vec2::new(size, size)),
                ..default()
            }
            .ease_to(
                Sprite {
                    color: Color::rgb(l[0] + 0.5, l[1] + 0.5, l[2] + 0.5),
                    custom_size: Some(Vec2::new(size * 1.2, size * 1.2)),
                    ..Default::default()
                },
                EaseFunction::SineInOut,
                EasingType::PingPong {
                    duration: Duration::from_millis(500),
                    pause: None,
                },
            ),
            collider: Collider::cuboid(size / 2.0 - 0.1, size / 2.0 - 0.1),
            restitution: Restitution::coefficient(1.0),
            velocity: Velocity {
                linvel: Vec2::new(dx, dy),
                angvel: 0.,
            },
            sprite: SpriteBundle {
                transform: Transform::from_xyz(x + dx * 0.2, y + dy * 0.2, 0.0),
                sprite: Sprite {
                    color: Color::rgb(0.75, 0.75, 0.75),
                    custom_size: Some(Vec2::new(size, size)),
                    ..default()
                },
                ..default()
            },
        }
    }

    fn from_vector(position: Vec2, size: f32) -> Self {
        Self::new(position.x, position.y, size)
    }
}

fn add_squares(mut particle_counter: ResMut<ParticleCount>, mut commands: Commands) {
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            ..default()
        },
        BloomSettings {
            intensity: 1.5,
            ..default()
        },
    ));
    commands.spawn(PositionedParticle::new(0.0, 200.0, 32.0));
    particle_counter.0 += 1;

    /* Create the ground. */
    commands
        .spawn(Collider::cuboid(500.0, 50.0))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, -300.0, 0.0)));
    commands
        .spawn(Collider::cuboid(500.0, 50.0))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, 300.0, 0.0)));

    // create walls
    commands
        .spawn(Collider::cuboid(50.0, 500.0))
        .insert(TransformBundle::from(Transform::from_xyz(-250.0, 0.0, 0.0)));

    commands
        .spawn(Collider::cuboid(50.0, 500.0))
        .insert(TransformBundle::from(Transform::from_xyz(250.0, 0.0, 0.0)));
}

pub struct SquaresPlugin;

impl Plugin for SquaresPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(add_squares);
    }
}
#[derive(Resource)]
struct Particles(i32);

fn mouse_button_events(
    mut commands: Commands,
    particles: Res<Particles>,
    mouse_input: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    mut particle_counter: ResMut<ParticleCount>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) {
    let window = windows.get_primary().unwrap();
    let (camera, camera_transform) = camera_q.single();

    if !mouse_input.pressed(MouseButton::Left) {
        return;
    }
    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
    {
        for _ in 0..particles.0 {
            commands.spawn(PositionedParticle::from_vector(
                world_position,
                rand::thread_rng().gen_range(1..8) as f32,
            ));
            particle_counter.0 += 1;
        }
    }
}

fn mouse_scroll_events(
    mut particles: ResMut<Particles>,
    mut scroll_event: EventReader<MouseWheel>,
) {
    for ev in scroll_event.iter() {
        particles.0 += if ev.y > 0.0 { 1 } else { -1 };
    }
}

#[derive(Resource)]
struct ParticleCount(u32);
fn show_particle_count(particles: Res<ParticleCount>) {
    println!("Particle count: {}", particles.0);
}
fn main() {
    let window_descriptor = WindowDescriptor {
        transparent: false,
        width: 800.0,
        height: 600.0,
        ..default()
    };

    App::new()
        .insert_resource(ClearColor(Color::hex("161616").unwrap()))
        .insert_resource(ParticleCount(0))
        .insert_resource(Particles(1))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: window_descriptor,
            ..default()
        }))
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(WorldInspectorPlugin)
        .add_plugin(EasingsPlugin)
        .add_plugin(SquaresPlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(1000.0))
        // .add_plugin(RapierDebugRenderPlugin::default())
        .add_system(mouse_button_events)
        .add_system(mouse_scroll_events)
        .add_system(show_particle_count)
        .run();
}
