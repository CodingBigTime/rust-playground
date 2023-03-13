extern crate uom;

use std::time::Duration;

use bevy::{
    core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping},
    input::mouse::*,
    prelude::*,
    window::{PrimaryWindow, WindowResolution},
};
use bevy_easings::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_prototype_lyon::{draw::Fill, entity::ShapeBundle, prelude::*, shapes::Circle};
use bevy_rapier2d::{plugin::*, prelude::*};
use rand::prelude::*;
use uom::{
    si,
    si::{
        area, energy, heat_capacity, length, mass, mass_density, specific_heat_capacity,
        temperature_interval, thermal_conductance, thermal_conductivity, thermodynamic_temperature,
        time, volume,
    },
};

trait ThermodynamicTemperatureToTemperatureIntervalConversion {
    fn as_temperature_interval(&self) -> si::f64::TemperatureInterval;
}

impl ThermodynamicTemperatureToTemperatureIntervalConversion for si::f64::ThermodynamicTemperature {
    fn as_temperature_interval(&self) -> si::f64::TemperatureInterval {
        si::f64::TemperatureInterval::new::<temperature_interval::kelvin>(
            self.get::<thermodynamic_temperature::kelvin>(),
        )
    }
}

#[derive(Component, Reflect)]
struct Material {
    #[reflect(ignore)]
    thermal_conductivity: si::f64::ThermalConductivity,
    #[reflect(ignore)]
    specific_heat_capacity: si::f64::SpecificHeatCapacity,
    #[reflect(ignore)]
    density: si::f64::MassDensity,
    base_color: Color,
}

enum MaterialType {
    Aluminium,
    Copper,
    Iron,
}

impl Material {
    fn new(
        thermal_conductivity: si::f64::ThermalConductivity,
        specific_heat_capacity: si::f64::SpecificHeatCapacity,
        density: si::f64::MassDensity,
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

impl From<MaterialType> for Material {
    fn from(material_type: MaterialType) -> Self {
        match material_type {
            MaterialType::Aluminium => Material::new(
                si::f64::ThermalConductivity::new::<thermal_conductivity::watt_per_meter_kelvin>(
                    237.0,
                ),
                si::f64::SpecificHeatCapacity::new::<
                    specific_heat_capacity::joule_per_kilogram_kelvin,
                >(0.9),
                si::f64::MassDensity::new::<mass_density::kilogram_per_cubic_meter>(2.7),
                Color::rgb(0.8, 0.8, 0.9),
            ),
            MaterialType::Copper => Material::new(
                si::f64::ThermalConductivity::new::<thermal_conductivity::watt_per_meter_kelvin>(
                    385.0,
                ),
                si::f64::SpecificHeatCapacity::new::<
                    specific_heat_capacity::joule_per_kilogram_kelvin,
                >(0.385),
                si::f64::MassDensity::new::<mass_density::kilogram_per_cubic_meter>(8.96),
                Color::rgb(0.9, 0.6, 0.2),
            ),
            MaterialType::Iron => Material::new(
                si::f64::ThermalConductivity::new::<thermal_conductivity::watt_per_meter_kelvin>(
                    80.0,
                ),
                si::f64::SpecificHeatCapacity::new::<
                    specific_heat_capacity::joule_per_kilogram_kelvin,
                >(0.45),
                si::f64::MassDensity::new::<mass_density::kilogram_per_cubic_meter>(7.87),
                Color::rgb(0.8, 0.8, 0.8),
            ),
        }
    }
}

#[derive(Component, Reflect)]
struct HeatBody {
    #[reflect(ignore)]
    heat: si::f64::Energy,
    #[reflect(ignore)]
    volume: si::f64::Volume,
    material: Material,
}

impl HeatBody {
    fn mass(&self) -> si::f64::Mass {
        self.volume * self.material.density
    }

    fn heat_capacity(&self) -> si::f64::HeatCapacity {
        self.material.specific_heat_capacity * self.mass()
    }

    fn temperature(&self) -> si::f64::ThermodynamicTemperature {
        si::f64::ThermodynamicTemperature::new::<thermodynamic_temperature::kelvin>(0.0)
            + self.heat / self.heat_capacity()
    }

    fn from_temperature_volume_material(
        temperature: si::f64::ThermodynamicTemperature,
        volume: si::f64::Volume,
        material: Material,
    ) -> Self {
        let heat: si::f64::Energy =
            temperature * material.specific_heat_capacity * volume * material.density;
        Self {
            heat,
            volume,
            material,
        }
    }

    fn add_heat(&mut self, heat: si::f64::Energy) {
        self.heat += heat;
    }

    fn add_temperature(&mut self, temperature: si::f64::TemperatureInterval) {
        self.add_heat(temperature * self.heat_capacity());
    }

    fn transfer_heat(&mut self, other: &mut Self, delta: Duration) {
        let time_delta: si::f64::Time = si::f64::Time::new::<time::second>(delta.as_secs_f64());
        let temperature_delta: si::f64::TemperatureInterval =
            self.temperature().as_temperature_interval()
                - other.temperature().as_temperature_interval();
        let mid_point_temperature: si::f64::ThermodynamicTemperature =
            self.temperature() - temperature_delta / 2.0;
        let disk_area: si::f64::Area = si::f64::Area::new::<area::square_millimeter>(1.0);
        let disk_thickness: si::f64::Length = si::f64::Length::new::<length::millimeter>(1.0);
        let thermal_conductance: si::f64::ThermalConductance =
            self.material.thermal_conductivity * disk_area / disk_thickness;
        let heat_transfer: si::f64::Energy = thermal_conductance * temperature_delta * time_delta;
        // the heat transfer shouldn't be more than the mid point temperature
        let heat_transfer = heat_transfer.min(mid_point_temperature * self.heat_capacity());
        let heat_transfer = heat_transfer.max(-(mid_point_temperature * other.heat_capacity()));

        self.add_heat(-heat_transfer);
        other.add_heat(heat_transfer);
    }
}

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

impl PositionedParticle {
    fn new(
        x: f32,
        y: f32,
        diameter: si::f64::Length,
        temperature: si::f64::ThermodynamicTemperature,
    ) -> Self {
        let mut rng = thread_rng();
        let angle = rng.gen_range(0.0..2. * std::f32::consts::PI);
        let dx = angle.sin() * 100.0;
        let dy = angle.cos() * 100.0;
        let temperature_kelvin = temperature.get::<thermodynamic_temperature::kelvin>() as f64;
        let diameter_millimeters = diameter.get::<length::millimeter>() as f32;
        let multiplier = color_multiplier(temperature_kelvin as f32);
        let rgb = colortemp::temp_to_rgb(temperature_kelvin as i64);
        Self {
            rigid_body: RigidBody::Dynamic,
            collider: Collider::ball(diameter_millimeters / 2.0 - 0.1),
            restitution: Restitution::coefficient(1.0),
            velocity: Velocity {
                linvel: Vec2::new(dx, dy),
                angvel: 0.,
            },
            sprite: (
                ShapeBundle {
                    path: GeometryBuilder::new()
                        .add(&Circle {
                            radius: diameter_millimeters / 2.0,
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
            temperature: HeatBody::from_temperature_volume_material(
                temperature,
                diameter * diameter * diameter * std::f64::consts::PI / 6.0,
                Material::from(MaterialType::Copper),
            ),
            event: ActiveEvents::COLLISION_EVENTS,
        }
    }

    fn spawn(self, commands: &mut Commands) {
        commands.spawn(self);
    }

    fn spawn_with_sleep_disabled(self, commands: &mut Commands) {
        commands.spawn(self).insert(Sleeping::disabled());
    }

    fn from_vector(
        position: Vec2,
        diameter: si::f64::Length,
        temperature: si::f64::ThermodynamicTemperature,
    ) -> Self {
        Self::new(position.x, position.y, diameter, temperature)
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
    PositionedParticle::new(
        0.0,
        200.0,
        si::f64::Length::new::<length::millimeter>(32.0),
        si::f64::ThermodynamicTemperature::new::<thermodynamic_temperature::kelvin>(1000.0),
    )
    .spawn_with_sleep_disabled(&mut commands);
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
            PositionedParticle::from_vector(
                world_position,
                si::f64::Length::new::<length::millimeter>(thread_rng().gen_range(1..16) as f64),
                if mouse_input.pressed(MouseButton::Left) {
                    si::f64::ThermodynamicTemperature::new::<thermodynamic_temperature::kelvin>(
                        thread_rng().gen_range(0.0..6000.0),
                    )
                } else {
                    si::f64::ThermodynamicTemperature::new::<thermodynamic_temperature::kelvin>(
                        thread_rng().gen_range(10000.0..100000.0),
                    )
                },
            )
            .spawn_with_sleep_disabled(&mut commands);
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

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
struct ParticleCount(u32);

fn heat_transfer_event(
    mut collision_events: EventReader<CollisionEvent>,
    mut query: Query<(&mut HeatBody, &mut Fill)>,
) {
    for event in collision_events.iter() {
        if let CollisionEvent::Started(a, b, _flags) = event {
            if !query.contains(*a) || !query.contains(*b) {
                continue;
            }
            let [entity_a, entity_b] = query.get_many_mut::<2>([*a, *b]).unwrap();
            let (mut heat_component_a, mut fill_a) = entity_a;
            let (mut heat_component_b, mut fill_b) = entity_b;

            println!(
                "Before: temps: {} {}, heat: {} {}",
                heat_component_a
                    .temperature()
                    .get::<thermodynamic_temperature::kelvin>(),
                heat_component_b
                    .temperature()
                    .get::<thermodynamic_temperature::kelvin>(),
                heat_component_a.heat.get::<energy::joule>(),
                heat_component_b.heat.get::<energy::joule>()
            );
            heat_component_a
                .transfer_heat(&mut heat_component_b, Duration::from_secs_f32(1.0 / 144.0));

            let rgb_a = colortemp::temp_to_rgb(
                heat_component_a
                    .temperature()
                    .get::<thermodynamic_temperature::kelvin>() as i64,
            );
            let rgb_b = colortemp::temp_to_rgb(
                heat_component_b
                    .temperature()
                    .get::<thermodynamic_temperature::kelvin>() as i64,
            );
            let multiplier_a = color_multiplier(
                heat_component_a
                    .temperature()
                    .get::<thermodynamic_temperature::kelvin>() as f32,
            );
            let multiplier_b = color_multiplier(
                heat_component_b
                    .temperature()
                    .get::<thermodynamic_temperature::kelvin>() as f32,
            );
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
                "After: temps: {} {}, heat: {} {}",
                heat_component_a
                    .temperature()
                    .get::<thermodynamic_temperature::kelvin>(),
                heat_component_b
                    .temperature()
                    .get::<thermodynamic_temperature::kelvin>(),
                heat_component_a.heat.get::<energy::joule>(),
                heat_component_b.heat.get::<energy::joule>()
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

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
struct PerformanceInfo {
    current_fps: f64,
    average_fps_10_frames: f64,
    average_fps_60_frames: f64,
    current_frame_time: Duration,
    average_frame_time_10_frames: Duration,
    average_frame_time_60_frames: Duration,
}

fn update_performance_info(time: Res<Time>, mut performance_info: ResMut<PerformanceInfo>) {
    // Update the performance info using Bevy's Time resource
    let current = time.delta();
    let average_10_frames = (performance_info.average_frame_time_10_frames * 9 + current) / 10;
    let average_60_frames = (performance_info.average_frame_time_60_frames * 59 + current) / 60;
    let fps = 1.0 / current.as_secs_f64();
    let fps_10_frames = 1.0 / average_10_frames.as_secs_f64();
    let fps_60_frames = 1.0 / average_60_frames.as_secs_f64();

    performance_info.current_frame_time = current;
    performance_info.average_frame_time_10_frames = average_10_frames;
    performance_info.average_frame_time_60_frames = average_60_frames;
    performance_info.current_fps = fps;
    performance_info.average_fps_10_frames = fps_10_frames;
    performance_info.average_fps_60_frames = fps_60_frames;
}
fn main() {
    App::new()
        .add_startup_system(setup)
        .insert_resource(ClearColor(Color::hex("161616").unwrap()))
        .insert_resource(ParticleCount::default())
        .insert_resource(Particles(1))
        .insert_resource(Msaa::Sample4)
        .insert_resource(PerformanceInfo::default())
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
        .register_type::<PerformanceInfo>()
        .register_type::<HeatBody>()
        .register_type::<ParticleCount>()
        .add_plugin(WorldInspectorPlugin::default())
        // .add_plugin(RapierDebugRenderPlugin::default())
        // .add_system(show_particle_count)
        .add_system(update_performance_info)
        .add_system(mouse_button_events)
        .add_system(mouse_scroll_events)
        .add_system(heat_transfer_event)
        .run();
}
