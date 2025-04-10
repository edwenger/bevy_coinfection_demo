use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use rand::prelude::*;

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
        .add_systems(Update, simulation_speed_slider)
        .run();
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

#[derive(Resource)]
struct Params {
    duration_liver: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            duration_liver: 7.0,
        }
    }
}

#[derive(Component)]
struct Host;

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
    params: Res<Params>,
    sim_time: Res<SimulationTime>,
) {
    let host_count = 5;

    for i in 0..host_count {
        let x = i as f32 * 100.0 - 200.0;

        // Spawn Host with Inoculation
        commands
            .spawn((
                Host,
                SpatialBundle {
                    transform: Transform::from_xyz(x, 0.0, 0.0),
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Inoculation {
                        state: InfectionState::E,
                        start_day: sim_time.day,
                        delay_days: params.duration_liver, // 7.0
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

    // Add a 2D camera
    commands.spawn(Camera2dBundle::default());
}

fn update_inoculations(sim_time: Res<SimulationTime>, mut query: Query<&mut Inoculation>) {
    for mut inoc in query.iter_mut() {
        if inoc.state == InfectionState::E {
            let elapsed = sim_time.day as f32 - inoc.start_day as f32;

            if elapsed >= inoc.delay_days {
                inoc.state = InfectionState::A;
                // possibly trigger treatment, broadcast, etc.
            }
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

fn simulation_speed_slider(mut contexts: EguiContexts, mut speed: ResMut<SimulationSpeed>) {
    egui::Window::new("Simulation Controls")
        .default_pos(egui::pos2(10.0, 50.0)) // ⬅ Move down to avoid overlap with time text
        .show(contexts.ctx_mut(), |ui| {
            ui.label("Simulation Speed");

            let mut multiplier = speed.multiplier;
            let response = ui.add(egui::Slider::new(&mut multiplier, 0.5..=5.0).text("x"));

            if response.changed() {
                speed.multiplier = multiplier;
            }
        });
}
