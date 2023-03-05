use std::time::Duration;

use bevy::core_pipeline::bloom::BloomSettings;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::input::mouse::*;
use bevy::prelude::*;
use bevy_easings::*;
use bevy_prototype_lyon::draw::{DrawMode, FillMode};
use bevy_prototype_lyon::entity::ShapeBundle;
use bevy_prototype_lyon::prelude::*;
use bevy_prototype_lyon::shapes::Circle;
use bevy_rapier2d::plugin::*;
use bevy_rapier2d::prelude::*;
use rand::prelude::*;

#[derive(Bundle)]
struct PositionedParticle {
    rigid_body: RigidBody,
    draw_mode_wrapper: DrawModeWrapper,
    collider: Collider,
    restitution: Restitution,
    velocity: Velocity,
    // color_easing: EasingComponent<DrawModeWrapper>,

    #[bundle]
    sprite: ShapeBundle,
}

#[derive(Component)]
struct DrawModeWrapper(DrawMode);

impl DrawModeWrapper {
    fn from_fill_mode(fill_mode: FillMode) -> Self {
        Self(DrawMode::Fill(fill_mode))
    }
}

impl Default for DrawModeWrapper {
    fn default() -> Self {
        Self(DrawMode::Fill(FillMode {
            options: Default::default(),
            color: Color::WHITE,
        }))
    }
}

impl Lerp for DrawModeWrapper {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        if let (DrawMode::Fill(fill), DrawMode::Fill(fill_other)) = (self.0, other.0) {
            return Self(DrawMode::Fill(FillMode {
                color: Color::Rgba {
                    red: fill.color.r().lerp(&fill_other.color.r(), scalar),
                    green: fill.color.g().lerp(&fill_other.color.g(), scalar),
                    blue: fill.color.b().lerp(&fill_other.color.b(), scalar),
                    alpha: fill.color.a().lerp(&fill_other.color.a(), scalar),
                },
                options: fill.options,
            })
            );
        }
        Self(self.0)
    }
}


impl PositionedParticle {
    fn new(x: f32, y: f32, size: f32) -> Self {
        let mut rng = thread_rng();
        let angle = rng.gen_range(0.0..2. * std::f32::consts::PI);
        let dx = angle.sin() * 100.0;
        let dy = angle.cos() * 100.0;
        // let mut rgb_list = [0.75, 0.25, 0.25];
        // rgb_list.shuffle(&mut rng);
        let rgb_list = [rng.gen_range(0.0..0.5), rng.gen_range(0.0..0.25), rng.gen_range(0.0..0.1)];
        Self {
            rigid_body: RigidBody::Dynamic,
            collider: Collider::ball(size / 2.0 - 0.1),
            restitution: Restitution::coefficient(1.0),
            velocity: Velocity {
                linvel: Vec2::new(dx, dy),
                angvel: 0.,
            },
            sprite: GeometryBuilder::build_as(
                &Circle {
                    radius: size / 2.0,
                    ..default()
                },
                DrawMode::Fill(FillMode::color(Color::rgb(0.92 + rgb_list[0], 0.7 + rgb_list[1], 0.18 + rgb_list[2]))),
                // DrawMode::Fill(FillMode::color(Color::WHITE)),
                Transform::from_xyz(x + dx * 0.2, y + dy * 0.2, 0.0),
            ),
            draw_mode_wrapper: DrawModeWrapper::default(),
            // color_easing: DrawModeWrapper::from_fill_mode(FillMode::color(Color::rgb(0.92 + rgb_list[0], 0.7 + rgb_list[1], 0.18 + rgb_list[2]))).ease_to(
            //     DrawModeWrapper::from_fill_mode(FillMode::color(Color::rgb(rgb_list[0] + 0.5, rgb_list[1] + 0.5, rgb_list[2] + 0.5))),
            //     EaseFunction::SineInOut,
            //     EasingType::PingPong {
            //         duration: Duration::from_millis(500),
            //         pause: None,
            //     },
            // ),
        }
    }

    fn from_vector(position: Vec2, size: f32) -> Self {
        Self::new(position.x, position.y, size)
    }
}

fn update_color(mut query: Query<(&mut DrawMode, &DrawModeWrapper)>) {
    for (mut draw_mode, draw_mode_wrapper) in query.iter_mut() {
        *draw_mode = draw_mode_wrapper.0;
    }
}

fn setup(mut particle_counter: ResMut<ParticleCount>, mut commands: Commands) {
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
        .add_startup_system(setup)
        .insert_resource(ClearColor(Color::hex("161616").unwrap()))
        .insert_resource(ParticleCount(0))
        .insert_resource(Particles(1))
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: window_descriptor,
            ..default()
        }))
        .add_plugin(ShapePlugin)
        .add_plugin(EasingsPlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(1000.0))
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(bevy_inspector_egui::quick::WorldInspectorPlugin)
        // .add_plugin(RapierDebugRenderPlugin::default())
        // .add_system(show_particle_count)
        .add_system(mouse_button_events)
        .add_system(mouse_scroll_events)
        // .add_system(update_color)
        .add_system(custom_ease_system::<DrawModeWrapper>)
        .run();
}