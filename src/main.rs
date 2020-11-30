#![feature(future_poll_fn)]
#![windows_subsystem = "windows"]

mod asyncio;
mod bot;
mod client;
mod clone_all;
mod datatypes;
mod foods;
mod mobs;
mod packets;

use asyncio::*;
use chrono::Local;
use fltk::{
    app::App, button::Button, button::CheckButton, enums::Color, frame::Frame, group::Group,
    group::Scroll, group::Tabs, input::Input, input::IntInput, input::MultilineInput,
    prelude::ValuatorExt, valuator::HorNiceSlider, window::Window, GroupExt, InputExt, WidgetBase,
    WidgetExt,
};
use foods::FOODS;
use mobs::MOBS;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct Settings {
    sneak: bool,
    attack_mobs: Vec<i64>,
    eat_food: Vec<i64>,
    attack_speed: f32, // attacks/second
    weapon: u8,
    min_health: f32,
    eat_at: f32,
}

fn main() {
    // initialize static maps
    lazy_static::initialize(&MOBS);
    lazy_static::initialize(&FOODS);

    // initialize tokio's runtime
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let handle = runtime.handle();

    // initialize the settings structure

    let settings = Arc::new(Mutex::new(Settings {
        sneak: false,
        attack_mobs: Vec::new(),
        eat_food: Vec::new(),
        attack_speed: 0.0,
        weapon: 0,
        min_health: 6.0,
        eat_at: 10.0,
    }));

    let app = App::default().with_scheme(fltk::app::Scheme::Gtk);

    let mut window = Window::new(100, 100, 400, 600, "Minecraft AFK Bot");

    let tabs = Tabs::new(0, 0, 400, 600, "");

    let main_tab = Group::new(0, 25, 400, 570, "Main");
    let address_input = Input::new(120, 45, 265, 30, "Server Adress");
    let username_input = Input::new(120, 80, 265, 30, "Username");
    let mut connect_button = Button::new(30, 130, 340, 40, "Connect");
    let mut short_status = Frame::new(15, 180, 370, 30, "not connected");
    short_status.set_label_color(Color::Dark3);
    let mut log = MultilineInput::new(15, 220, 370, 365, "");
    log.set_readonly(true);
    log.set_wrap(true);
    main_tab.end();

    // this is a convenience closure for easy writing to the log widget
    let mut write_to_log = move |message: String| {
        let mut text = log.value();
        text = text + &format!("[{}] {}\n", Local::now().format("%F %T"), message);
        log.set_value(&text);
        log.set_position(text.len() as u32).unwrap();
    };

    // this is a convenience closure for easy status updates
    let change_status = move |message: String, color: Color| {
        short_status.set_label(&message);
        short_status.set_label_color(color);
    };

    let settings_tab = Group::new(0, 25, 400, 570, "Settings");
    let mut sneak_checkbox = CheckButton::new(15, 45, 360, 25, "Sneak");
    let mut min_hp_input = IntInput::new(250, 80, 120, 25, "Disconnect when HP below: ");
    min_hp_input.set_value("6");
    min_hp_input.set_maximum_size(2);
    Frame::new(15, 110, 360, 20, "2 HP = 1 HEART").set_label_color(Color::Dark3);
    let mut eat_at_input = IntInput::new(250, 140, 120, 25, "Eat when hunger below: ");
    eat_at_input.set_value("10");
    min_hp_input.set_maximum_size(2);
    Frame::new(15, 175, 360, 20, "20 HUNGER = FULL BAR").set_label_color(Color::Dark3);
    Frame::new(15, 220, 360, 20, "Attack Speed");
    let mut attack_speed_slider = HorNiceSlider::new(15, 240, 360, 30, "0.0 attacks/second");
    attack_speed_slider.set_bounds(0.0, 4.472); // the upper bound is sqrt(20)
    attack_speed_slider.set_precision(1);
    Frame::new(15, 310, 360, 20, "Weapon");
    let mut weapon_slider = HorNiceSlider::new(15, 330, 360, 30, "1");
    weapon_slider.set_bounds(1.0, 9.0);
    weapon_slider.set_precision(0);
    Frame::new(15, 380, 360, 20, "choose a hotbar slot 1-9").set_label_color(Color::Dark3);
    settings_tab.end();

    let mobs_tab = Scroll::new(0, 25, 400, 570, "Mobs");
    Frame::new(15, 40, 360, 20, "select all mobs you wish to attack").set_label_color(Color::Dark3);
    let mut i = 0;
    for mob in MOBS.right_values() {
        let mut checkbox = CheckButton::new(15 + (i % 2) * 170, 65 + (i / 2) * 20, 155, 20, mob);

        checkbox.set_callback2({
            clone_all![handle, settings, write_to_log];
            move |checkbox| {
                handle.spawn({
                    clone_mut![write_to_log];
                    clone_all![settings, checkbox];
                    async move {
                        let is_checked = checkbox.is_checked();
                        let name = checkbox.label();
                        let id = *MOBS.get_by_right(&name.as_str()).unwrap();
                        if is_checked {
                            settings.lock().await.attack_mobs.push(id);
                        } else {
                            settings.lock().await.attack_mobs.retain(|&x| x != id);
                        }

                        write_to_log(format!(
                            "Mob {} {}.",
                            name,
                            if is_checked { "selected" } else { "unselected" }
                        ));
                    }
                });
            }
        });

        i += 1;
    }
    mobs_tab.end();

    let food_tab = Scroll::new(0, 25, 400, 570, "Food");
    Frame::new(15, 40, 360, 60, "select all food you wish to automatically eat\nwhen hungry. The bot can only eat food\nthat's in the hotbar, so make sure to have some.").set_label_color(Color::Dark3);
    let mut i = 0;
    for food in FOODS.right_values() {
        let mut checkbox = CheckButton::new(15 + (i % 2) * 170, 110 + (i / 2) * 20, 155, 20, food);

        checkbox.set_callback2({
            clone_all![handle, settings, write_to_log];
            move |checkbox| {
                handle.spawn({
                    clone_mut![write_to_log];
                    clone_all![settings, checkbox];
                    async move {
                        let is_checked = checkbox.is_checked();
                        let name = checkbox.label();
                        let id = *FOODS.get_by_right(&name.as_str()).unwrap();
                        if is_checked {
                            settings.lock().await.eat_food.push(id);
                        } else {
                            settings.lock().await.eat_food.retain(|&x| x != id);
                        }

                        write_to_log(format!(
                            "Food {} {}.",
                            name,
                            if is_checked { "selected" } else { "unselected" }
                        ));
                    }
                });
            }
        });

        i += 1;
    }
    food_tab.end();

    let statistics_tab = Scroll::new(0, 40, 400, 570, "Statistics");
    let mut statistics_frame = Frame::new(15, 40, 370, 530, "");

    // this is a convenience closure for easy statistics updates
    let mut update_statistics = move |statistics: String| {
        statistics_frame.resize(15, 40, 370, 20 * (statistics.lines().count() as i32 + 1));
        statistics_frame.set_label(&statistics);
    };
    update_statistics(format!("Nothing to show yet."));

    statistics_tab.end();

    tabs.end();

    window.end();

    window.show();

    // GUI callback closures follow

    sneak_checkbox.set_callback2({
        clone_all![handle, settings, write_to_log];
        move |sneak_checkbox| {
            handle.spawn({
                clone_mut![write_to_log];
                clone_all![settings, sneak_checkbox];
                async move {
                    let new = sneak_checkbox.is_checked();
                    settings.lock().await.sneak = new;
                    write_to_log(format!("Changed sneak to {}", new));
                }
            });
        }
    });

    min_hp_input.set_callback2({
        clone_all![handle, settings, write_to_log];
        move |min_hp_input| {
            handle.spawn({
                clone_mut![write_to_log];
                clone_all![settings, min_hp_input];
                async move {
                    let new = min_hp_input.value();
                    let new = if new.len() == 0 {
                        0.0
                    } else {
                        new.parse().unwrap_or(6.0) // 6 - default
                    };
                    settings.lock().await.min_health = new;
                    write_to_log(format!("Changed min HP to {}", new));
                }
            });
        }
    });

    eat_at_input.set_callback2({
        clone_all![handle, settings, write_to_log];
        move |eat_at_input| {
            handle.spawn({
                clone_mut![write_to_log];
                clone_all![settings, eat_at_input];
                async move {
                    let new = eat_at_input.value();
                    let new = if new.len() == 0 {
                        0.0
                    } else {
                        new.parse().unwrap_or(6.0) // 6 - default
                    };
                    settings.lock().await.eat_at = new;
                    write_to_log(format!("Changed minimum hunger to {}", new));
                }
            });
        }
    });

    attack_speed_slider.set_callback2({
        clone_all![handle, settings];
        move |attack_speed_slider| {
            handle.spawn({
                clone_mut![attack_speed_slider];
                clone_all![settings];
                async move {
                    let new = attack_speed_slider.value();
                    // make it non-linear
                    let new = (new.powi(2) * 10.0).ceil() / 10.0;
                    settings.lock().await.attack_speed = new as f32;
                    attack_speed_slider.set_label(&format!("  {:.1} attacks/second  ", new));
                }
            });
        }
    });

    weapon_slider.set_callback2({
        clone_all![handle, settings];
        move |weapon_slider| {
            handle.spawn({
                clone_mut![weapon_slider];
                clone_all![settings];
                async move {
                    let new = weapon_slider.value() as u8;
                    settings.lock().await.weapon = new - 1;
                    weapon_slider.set_label(&format!("  {:}  ", new));
                }
            });
        }
    });

    // when connected, holds a sender, which, when used, disconnects.
    let connected: Arc<Mutex<Option<tokio::sync::mpsc::Sender<()>>>> = Arc::new(Mutex::new(None));

    connect_button.set_callback2({
        clone_all![
            handle,
            settings,
            update_statistics,
            write_to_log,
            change_status,
            address_input,
            username_input
        ];
        move |connect_button| {
            handle.spawn({
                clone_mut![
                    write_to_log,
                    change_status,
                    address_input,
                    username_input,
                    connect_button
                ];
                clone_all![settings, connected, update_statistics];
                async move {
                    let mut connected_lock = connected.lock().await;
                    if let Some(sender) = &*connected_lock {
                        // disconnect
                        sender.send(()).await.unwrap();
                    } else {
                        // connect

                        // make sure an username is provided
                        // no need to check the address, because it will be checked at some point automatically
                        if username_input.value().len() == 0 {
                            write_to_log(format!("Please provide a username!"));
                            return;
                        }
                        address_input.set_readonly(true);
                        username_input.set_readonly(true);
                        connect_button.set_label("Disconnect");
                        let (sender, receiver) = mpsc::channel(1);
                        *connected_lock = Some(sender.clone());
                        drop(connected_lock);
                        write_to_log(format!(
                            "Connecting to {:?} as {:?}.",
                            address_input.value(),
                            username_input.value(),
                        ));
                        let res = {
                            clone_all![write_to_log, change_status];
                            bot::run(
                                address_input.value(),
                                username_input.value(),
                                settings,
                                write_to_log,
                                change_status,
                                receiver,
                                sender,
                                update_statistics,
                            )
                            .await
                        };
                        if let Err(e) = res {
                            write_to_log(format!("Error: {}", e));
                        }
                        *connected.lock().await = None;
                        change_status(format!("not connected"), Color::Dark3);
                        address_input.set_readonly(false);
                        username_input.set_readonly(false);
                        connect_button.set_label("Connect");
                        write_to_log(format!("Disconnected."));
                    }
                }
            });
        }
    });

    write_to_log(format!("Started Minecraft AFK Bot application."));
    // change_status(format!("Hello"), Color::DarkGreen);
    app.run().unwrap();
}
