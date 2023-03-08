use bevy::core_pipeline::bloom::BloomSettings;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::input::mouse::*;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy_easings::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_prototype_lyon::draw::Fill;
use bevy_prototype_lyon::entity::ShapeBundle;
use bevy_prototype_lyon::prelude::*;
use bevy_prototype_lyon::shapes::Circle;
use bevy_rapier2d::plugin::*;
use bevy_rapier2d::prelude::*;
use rand::prelude::*;

#[derive(Bundle)]
struct PositionedParticle {
    rigid_body: RigidBody,
    collider: Collider,
    restitution: Restitution,
    velocity: Velocity,

    #[bundle]
    sprite: (ShapeBundle, Fill),
}

impl PositionedParticle {
    fn new(x: f32, y: f32, size: f32) -> Self {
        let mut rng = thread_rng();
        let angle = rng.gen_range(0.0..2. * std::f32::consts::PI);
        let dx = angle.sin() * 100.0;
        let dy = angle.cos() * 100.0;
        let rgb_list = [rng.gen_range(0.0..0.5), rng.gen_range(0.0..0.25), rng.gen_range(0.0..0.1)];
        let multiplier = 4.0;
        Self {
            rigid_body: RigidBody::Dynamic,
            collider: Collider::ball(size / 2.0 - 0.1),
            restitution: Restitution::coefficient(1.0),
            velocity: Velocity {
                linvel: Vec2::new(dx, dy),
                angvel: 0.,
            },
            sprite: (
                ShapeBundle {
                    path: GeometryBuilder::new().add(
                        &Circle {
                            radius: size / 2.0,
                            ..default()
                        },
                    ).build(),
                    transform: Transform::from_xyz(x + dx * 0.2, y + dy * 0.2, 0.0),
                    ..default()
                },
                Fill::color(Color::rgb((0.92 + rgb_list[0]) * multiplier, (0.7 + rgb_list[1]) * multiplier, (0.18 + rgb_list[2]) * multiplier)),
            ),
        }
    }

    fn from_vector(position: Vec2, size: f32) -> Self {
        Self::new(position.x, position.y, size)
    }
}

fn setup(mut particle_counter: ResMut<ParticleCount>, mut commands: Commands) {
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            tonemapping: Tonemapping::TonyMcMapface,
            ..default()
        },
        BloomSettings {
            low_frequency_boost: 0.5,
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

#[derive(Resource)]
struct Particles(i32);

fn mouse_button_events(
    mut commands: Commands,
    particles: Res<Particles>,
    mouse_input: Res<Input<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut particle_counter: ResMut<ParticleCount>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };
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
                thread_rng().gen_range(1..16) as f32,
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
        particles.0 = (particles.0 + if ev.y > 0.0 { 1 } else { -1 }).max(1);
    }
}

#[derive(Resource)]
struct ParticleCount(u32);

fn show_particle_count(particles: Res<ParticleCount>) {
    println!("Particle count: {}", particles.0);
}

fn main() {
    App::new()
        .add_startup_system(setup)
        .insert_resource(ClearColor(Color::hex("161616").unwrap()))
        .insert_resource(ParticleCount(0))
        .insert_resource(Particles(1))
        .insert_resource(Msaa::Sample4)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                transparent: false,
                resolution: WindowResolution::new(800.0, 600.0),
                ..default()
            }),
            ..default()
        }))
        .add_plugin(ShapePlugin)
        .add_plugin(EasingsPlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(1000.0))
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(WorldInspectorPlugin::default())
        // .add_plugin(RapierDebugRenderPlugin::default())
        // .add_system(show_particle_count)
        .add_system(mouse_button_events)
        .add_system(mouse_scroll_events)
        .run();
}
