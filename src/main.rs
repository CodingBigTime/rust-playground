use std::time::Duration;

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
    temperature: HeatBody,
    event: ActiveEvents,

    #[bundle]
    sprite: (ShapeBundle, Fill),
}

type Joules = f32;
type JoulesPerKelvin = f32;
type JoulesPerKelvinPerKilogram = f32;
type WattsPerMetreKelvin = f32;
type KiloGram = f32;
type KiloGramPerMetreCubed = f32;
type CubicMetres = f32;

enum Temperature {
    Kelvin(f32),
    Celsius(f32),
    Fahrenheit(f32),
}

impl Default for Temperature {
    fn default() -> Self {
        Self::Kelvin(0f32)
    }
}

impl Into<f32> for Temperature {
    fn into(self) -> f32 {
        match self {
            Self::Kelvin(kelvin) => kelvin,
            Self::Celsius(celsius) => celsius + 273.15,
            Self::Fahrenheit(fahrenheit) => (fahrenheit - 32.0) * 5.0 / 9.0 + 273.15,
        }
    }
}

impl std::fmt::Display for Temperature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Kelvin(kelvin) => write!(f, "{}°K", kelvin),
            Self::Celsius(celsius) => write!(f, "{}°C", celsius),
            Self::Fahrenheit(fahrenheit) => write!(f, "{}°F", fahrenheit),
        }
    }
}

impl Temperature {
    const ABSOLUTE_ZERO: Temperature = Temperature::Kelvin(0.0);
    const WATER_MELTING_POINT: Temperature = Temperature::Celsius(0.0);
    const WATER_BOILING_POINT: Temperature = Temperature::Celsius(100.0);
    const HUMAN_BODY_TEMPERATURE: Temperature = Temperature::Celsius(37.0);
    const SUN_SURFACE_TEMPERATURE: Temperature = Temperature::Celsius(5500.0);
    const SUN_CORE_TEMPERATURE: Temperature = Temperature::Celsius(15_000_000.0);
    const ROOM_TEMPERATURE: Temperature = Temperature::Celsius(20.0);

    fn as_kelvin_f32(&self) -> f32 {
        match self {
            Self::Kelvin(kelvin) => *kelvin,
            Self::Celsius(celsius) => celsius + 273.15,
            Self::Fahrenheit(fahrenheit) => (fahrenheit - 32.0) * 5.0 / 9.0 + 273.15,
        }
    }

    fn as_kelvin(&self) -> Temperature {
        Self::Kelvin(self.as_kelvin_f32())
    }

    fn as_celsius_f32(&self) -> f32 {
        match self {
            Self::Kelvin(kelvin) => kelvin - 273.15,
            Self::Celsius(celsius) => *celsius,
            Self::Fahrenheit(fahrenheit) => (fahrenheit - 32.0) * 5.0 / 9.0,
        }
    }

    fn as_celsius(&self) -> Temperature {
        Self::Celsius(self.as_celsius_f32())
    }

    fn as_fahrenheit_f32(&self) -> f32 {
        match self {
            Self::Kelvin(kelvin) => (kelvin - 273.15) * 9.0 / 5.0 + 32.0,
            Self::Celsius(celsius) => celsius * 9.0 / 5.0 + 32.0,
            Self::Fahrenheit(fahrenheit) => *fahrenheit,
        }
    }

    fn as_fahrenheit(&self) -> Temperature {
        Self::Fahrenheit(self.as_fahrenheit_f32())
    }
}

struct Material {
    thermal_conductivity: WattsPerMetreKelvin,
    specific_heat_capacity: JoulesPerKelvinPerKilogram,
    density: KiloGramPerMetreCubed,
    base_color: Color,
}

impl Material {
    const ALUMINIUM: Material = Material {
        thermal_conductivity: 237.0,
        specific_heat_capacity: 0.9,
        density: 2.7,
        base_color: Color::rgb(0.8, 0.8, 0.9),
    };
    const COPPER: Material = Material {
        thermal_conductivity: 385.0,
        specific_heat_capacity: 0.385,
        density: 8.96,
        base_color: Color::rgb(0.9, 0.6, 0.2),
    };
    const IRON: Material = Material {
        thermal_conductivity: 80.0,
        specific_heat_capacity: 0.45,
        density: 7.87,
        base_color: Color::rgb(0.8, 0.8, 0.8),
    };

    fn new(
        thermal_conductivity: f32,
        specific_heat_capacity: f32,
        density: KiloGramPerMetreCubed,
        base_color: Color,
    ) -> Self {
        Material {
            thermal_conductivity,
            specific_heat_capacity,
            density,
            base_color,
        }
    }
}

#[derive(Component)]
struct HeatBody {
    heat: Joules,
    size: CubicMetres,
    material: Material,
}

impl HeatBody {
    fn mass(&self) -> KiloGram {
        self.size * self.material.density
    }

    fn heat_capacity(&self) -> JoulesPerKelvin {
        self.material.specific_heat_capacity * self.mass()
    }

    fn temperature(&self) -> Temperature {
        Temperature::Kelvin(self.heat / self.heat_capacity())
    }

    fn from_temperature_size_material(
        temperature: Temperature,
        size: CubicMetres,
        material: Material,
    ) -> Self {
        let heat =
            temperature.as_kelvin_f32() * material.specific_heat_capacity * size * material.density;
        Self {
            heat,
            size,
            material,
        }
    }

    fn add_heat(&mut self, heat: Joules) {
        self.heat += heat;
    }

    fn add_temperature(&mut self, temperature: Temperature) {
        self.add_heat(temperature.as_kelvin_f32() * self.heat_capacity());
    }

    fn transfer_heat(&mut self, other: &mut Self, delta: Duration) {
        let thermal_conductivity =
            (self.material.thermal_conductivity + other.material.thermal_conductivity) / 2.0;
        let heat_transfer = thermal_conductivity
            * (self.temperature().as_kelvin_f32() - other.temperature().as_kelvin_f32())
            * delta.as_secs_f32();
        self.add_heat(-heat_transfer);
        other.add_heat(heat_transfer);
    }
}

impl PositionedParticle {
    fn new(x: f32, y: f32, diameter: f32, temperature: Temperature) -> Self {
        let mut rng = thread_rng();
        let angle = rng.gen_range(0.0..2. * std::f32::consts::PI);
        let dx = angle.sin() * 100.0;
        let dy = angle.cos() * 100.0;
        let multiplier = color_multiplier(temperature.as_kelvin_f32());
        let rgb = colortemp::temp_to_rgb(temperature.as_kelvin_f32() as i64);
        Self {
            rigid_body: RigidBody::Dynamic,
            collider: Collider::ball(diameter / 2.0 - 0.1),
            restitution: Restitution::coefficient(1.0),
            velocity: Velocity {
                linvel: Vec2::new(dx, dy),
                angvel: 0.,
            },
            sprite: (
                ShapeBundle {
                    path: GeometryBuilder::new()
                        .add(&Circle {
                            radius: diameter / 2.0,
                            ..default()
                        })
                        .build(),
                    transform: Transform::from_xyz(x + dx * 0.2, y + dy * 0.2, 0.0),
                    ..default()
                },
                Fill::color(Color::rgb(
                    multiplier * rgb.r as f32 / 255.0,
                    multiplier * rgb.g as f32 / 255.0,
                    multiplier * rgb.b as f32 / 255.0,
                )),
            ),
            temperature: HeatBody::from_temperature_size_material(
                temperature,
                diameter * diameter * diameter * std::f32::consts::PI / 6.0,
                Material::COPPER,
            ),
            event: ActiveEvents::COLLISION_EVENTS,
        }
    }

    fn from_vector(position: Vec2, size: f32, temperature: Temperature) -> Self {
        Self::new(position.x, position.y, size, temperature)
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
    commands.spawn(PositionedParticle::new(
        0.0,
        200.0,
        32.0,
        Temperature::Kelvin(1000.0),
    ));
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

    if !mouse_input.pressed(MouseButton::Left) && !mouse_input.pressed(MouseButton::Right) {
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
                if mouse_input.pressed(MouseButton::Left) {
                    Temperature::Kelvin(thread_rng().gen_range(0.0..6000.0))
                } else {
                    Temperature::Kelvin(thread_rng().gen_range(10000.0..1000000.0))
                },
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

fn heat_transfer_event(
    mut collision_events: EventReader<CollisionEvent>,
    mut query: Query<(&mut HeatBody, &mut Fill)>,
) {
    for event in collision_events.iter() {
        if let CollisionEvent::Started(a, b, flags) = event {
            if !query.contains(*a) || !query.contains(*b) {
                continue;
            }
            let [entity_a, entity_b] = query.get_many_mut::<2>([*a, *b]).unwrap();
            let (mut heat_component_a, mut fill_a) = entity_a;
            let (mut heat_component_b, mut fill_b) = entity_b;

            println!(
                "Before: {} {}",
                heat_component_a.temperature(),
                heat_component_b.temperature()
            );
            heat_component_a
                .transfer_heat(&mut heat_component_b, Duration::from_secs_f32(1.0 / 144.0));

            let rgb_a =
                colortemp::temp_to_rgb(heat_component_a.temperature().as_kelvin_f32() as i64);
            let rgb_b =
                colortemp::temp_to_rgb(heat_component_b.temperature().as_kelvin_f32() as i64);
            let multiplier_a = color_multiplier(heat_component_a.temperature().as_kelvin_f32());
            let multiplier_b = color_multiplier(heat_component_b.temperature().as_kelvin_f32());
            fill_a.color = Color::rgb(
                multiplier_a * rgb_a.r as f32 / 255.0,
                multiplier_a * rgb_a.g as f32 / 255.0,
                multiplier_a * rgb_a.b as f32 / 255.0,
            );
            fill_b.color = Color::rgb(
                multiplier_b * rgb_b.r as f32 / 255.0,
                multiplier_b * rgb_b.g as f32 / 255.0,
                multiplier_b * rgb_b.b as f32 / 255.0,
            );

            println!(
                "After: {} {}",
                heat_component_a.temperature(),
                heat_component_b.temperature()
            );
        }
    }
}

fn color_multiplier(temperature: f32) -> f32 {
    match temperature.log(4.0) {
        x if x.is_nan() => 1.0,
        x => x,
    }
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
        // .add_plugin(LogDiagnosticsPlugin::default())
        // .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(WorldInspectorPlugin::default())
        // .add_plugin(RapierDebugRenderPlugin::default())
        // .add_system(show_particle_count)
        .add_system(mouse_button_events)
        .add_system(mouse_scroll_events)
        .add_system(heat_transfer_event)
        .run();
}
