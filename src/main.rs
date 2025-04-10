use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy::window::PrimaryWindow;
use bevy::render::camera::ScalingMode;
use rand::distributions::{Uniform, Distribution};

fn main() {
    env_logger::init(); // initializes logging

    App::new()
        .insert_resource(ClearColor(Color::rgb(0.1, 0.1, 0.1)))
        .insert_resource(Params::default())
        .insert_resource(SimulationTime::default())
        .insert_resource(SimulationSpeed::default())
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (update_simulation_time, update_inoculations))
        .add_systems(Update, update_inoculation_visuals)
        .add_systems(Update, simulation_controls_ui)
        .add_systems(Update, spawn_infections)
        .add_systems(Update, update_inoculation_positions)
        .run();
}

#[derive(Resource)]
struct Params {
    duration_liver: f32,
    duration_prophylaxis: f32,
    prob_acute: f32,
    prob_ac: f32,
    prob_treatment: f32,
    duration_acute: Uniform<f32>,
    duration_chronic: Uniform<f32>,
    treatment_delay: Uniform<f32>,
    incidence_rate: f32, // New infections per time step
}

impl Default for Params {
    fn default() -> Self {
        Self {
            duration_liver: 7.0,
            duration_prophylaxis: 14.0,
            prob_acute: 0.7,
            prob_ac: 0.2,
            prob_treatment: 0.4,
            duration_acute: Uniform::new(10.0, 40.0),
            duration_chronic: Uniform::new(100.0, 400.0),
            treatment_delay: Uniform::new(0.0, 2.0),
            incidence_rate: 0.1,
        }
    }
}

#[derive(Resource)]
struct SimulationTime {
    day: u32,
    timer: Timer,
}

impl Default for SimulationTime {
    fn default() -> Self {
        Self {
            day: 0,
            timer: Timer::from_seconds(1.0, TimerMode::Repeating), // one day per second
        }
    }
}

#[derive(Resource)]
struct SimulationSpeed {
    multiplier: f32, // 1.0 by default
}

impl Default for SimulationSpeed {
    fn default() -> Self {
        Self { multiplier: 1.0 }
    }
}

#[derive(Component, Default)]
struct Host {
    on_prophylaxis: bool,
    treat_request_day: Option<u32>, // pending treatment
}

#[derive(Component)]
struct Inoculation {
    state: InfectionState,
    start_day: u32,
    delay_days: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InfectionState {
    E, // Exposed
    A, // Acute
    C, // Chronic
    S, // Cleared
}

#[derive(Component)]
struct TimeText;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    query: Query<&Window, With<PrimaryWindow>>, // Query for the primary window
    params: Res<Params>,
    sim_time: Res<SimulationTime>,
) {
    let window = query.single(); // Get the primary window

    let bottom_y = -window.height() / 4.0 + 40.0; // Adjusted to position hosts comfortably above the bottom edge

    let host_count = 5;
    let spacing = window.width() / (host_count as f32 + 1.0) / 2.0; // Dynamically calculate spacing based on window width

    for i in 0..host_count {
        let x = (i as f32 + 1.0) * spacing - window.width() / 4.0; // Distribute hosts evenly across the screen

        // Spawn Host with Inoculation
        commands
            .spawn((
                Host {
                    ..default()
                },
                SpatialBundle {
                    transform: Transform::from_xyz(x, bottom_y, 0.0),
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Inoculation {
                        state: InfectionState::E,
                        start_day: sim_time.day,
                        delay_days: params.duration_liver,
                    },
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::BLUE,
                            custom_size: Some(Vec2::splat(30.0)),
                            ..default()
                        },
                        transform: Transform::from_xyz(0.0, 0.0, 0.1),
                        ..default()
                    },
                ));
            });
    }

    // Add UI text
    commands.spawn((
        TimeText,
        TextBundle {
            text: Text::from_section(
                "t = 0",
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 30.0,
                    color: Color::WHITE,
                },
            ),
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                ..default()
            },
            ..default()
        },
    ));

    // Add a default 2D camera
    commands.spawn(Camera2dBundle {
        ..default()
    });
}

fn update_inoculations(
    mut commands: Commands,
    mut inoc_query: Query<(Entity, &mut Inoculation, &Parent)>,
    params: Res<Params>,
    sim_time: Res<SimulationTime>,
) {
    for (entity, mut inoc, parent) in inoc_query.iter_mut() {
        let days_elapsed = sim_time.day as f32 - inoc.start_day as f32;

        match inoc.state {
            InfectionState::E if days_elapsed >= inoc.delay_days => {
                let mut rng = rand::thread_rng();
                let goes_acute = rand::random::<f32>() < params.prob_acute;

                inoc.state = if goes_acute {
                    InfectionState::A
                } else {
                    InfectionState::C
                };

                inoc.start_day = sim_time.day;
                inoc.delay_days = if goes_acute {
                    params.duration_acute.sample(&mut rng)
                } else {
                    params.duration_chronic.sample(&mut rng)
                };
            }

            InfectionState::A if days_elapsed >= inoc.delay_days => {
                let goes_chronic = rand::random::<f32>() < params.prob_ac;
                if goes_chronic {
                    inoc.state = InfectionState::C;
                    inoc.start_day = sim_time.day;
                    inoc.delay_days = params.duration_chronic.sample(&mut rand::thread_rng());
                } else {
                    commands.entity(parent.get()).remove_children(&[entity]);
                    commands.entity(entity).despawn();
                }
            }

            InfectionState::C if days_elapsed >= inoc.delay_days => {
                commands.entity(parent.get()).remove_children(&[entity]);
                commands.entity(entity).despawn();
            }

            _ => {}
        }
    }
}

fn update_inoculation_visuals(mut inoc_query: Query<(&Inoculation, &mut Sprite)>) {
    for (inoc, mut sprite) in inoc_query.iter_mut() {
        sprite.color = match inoc.state {
            InfectionState::E => Color::BLUE,
            InfectionState::A => Color::RED,
            InfectionState::C => Color::ORANGE,
            InfectionState::S => Color::GREEN,
        };
    }
}

fn update_simulation_time(
    time: Res<Time>,
    speed: Res<SimulationSpeed>,
    mut sim_time: ResMut<SimulationTime>,
    mut text_query: Query<&mut Text, With<TimeText>>,
) {
    sim_time.timer.tick(time.delta().mul_f32(speed.multiplier));

    if sim_time.timer.just_finished() {
        sim_time.day += 1;
        for mut text in text_query.iter_mut() {
            text.sections[0].value = format!("t = {}", sim_time.day);
        }
    }
}

fn simulation_controls_ui(mut contexts: EguiContexts, mut params: ResMut<Params>, mut speed: ResMut<SimulationSpeed>) {
    egui::Window::new("Simulation Controls")
        .default_pos(egui::pos2(10.0, 50.0))
        .show(contexts.ctx_mut(), |ui| {
            ui.label("Simulation Speed");

            let mut param_value = speed.multiplier;
            let response = ui.add(egui::Slider::new(&mut param_value, 0.5..=5.0).text("Speed Multiplier"));

            if response.changed() {
                speed.multiplier = param_value;
            }

            ui.label("Incidence Rate");

            let mut param_value = params.incidence_rate;
            let response = ui.add(egui::Slider::new(&mut param_value, 0.0..=0.2).text("Incidence Rate"));

            if response.changed() {
                params.incidence_rate = param_value;
            }
        });
}

fn spawn_infections(
    mut commands: Commands,
    mut host_query: Query<(Entity, &Children)>,
    params: Res<Params>,
    sim_time: Res<SimulationTime>,
    time: Res<Time>,
    speed: Res<SimulationSpeed>,
) {

    for (host_entity, children) in host_query.iter_mut() {
        if rand::random::<f32>() < params.incidence_rate * time.delta_seconds() * speed.multiplier {
            // Calculate position for the new inoculation
            let y_offset = children.len() as f32 * 40.0; // Stack inoculations vertically

            // Spawn a new Inoculation as a child of the Host
            commands.entity(host_entity).with_children(|parent| {
                parent.spawn((
                    Inoculation {
                        state: InfectionState::E,
                        start_day: sim_time.day,
                        delay_days: params.duration_liver,
                    },
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::rgba(0.0, 0.0, 1.0, 0.0), // Transparent blue on spawn
                            custom_size: Some(Vec2::splat(30.0)),
                            ..default()
                        },
                        transform: Transform::from_xyz(0.0, y_offset, 0.1),
                        ..default()
                    },
                ));
            });
        }
    }
}

fn update_inoculation_positions(
    host_query: Query<(&Children, &Transform), With<Host>>,
    mut inoc_query: Query<&mut Transform, (With<Inoculation>, Without<Host>)>,
) {
    for (children, host_transform) in host_query.iter() {
        for (index, &child) in children.iter().enumerate() {
            if let Ok(mut inoc_transform) = inoc_query.get_mut(child) {
                inoc_transform.translation = host_transform.translation + Vec3::new(0.0, index as f32 * 40.0, 0.1);
            }
        }
    }
}
