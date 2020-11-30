use crate::client::Client;
use crate::datatypes::*;
use crate::mobs::MOBS;
use crate::packets::*;
use crate::Settings;
use crate::{clone_all, clone_mut};
use fltk::Color;
use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use tokio::io;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

#[derive(Debug)]
struct State {
    shutdown: bool,
    my_entity_id: Option<VarInt>,
    my_pos: (f64, f64, f64),
    mobs: HashMap<VarInt, Mob>,
    hotbar: [Slot; 9],
    sneaking: bool,
    held_item: u8,
    food: f32,
    initial_statistics: Vec<(VarInt, VarInt, VarInt)>,
    statistics: Vec<(VarInt, VarInt, VarInt)>,
}

#[derive(Debug)]
struct Mob {
    entity_id: VarInt,
    entity_type: VarInt,
    pos: (f64, f64, f64),
}

pub async fn run<W, C, S>(
    ip: String,
    username: String,
    settings: Arc<Mutex<Settings>>,
    write_to_log: W,
    change_status: C,
    mut shutdown_receiver: mpsc::Receiver<()>,
    shutdown_sender: mpsc::Sender<()>,
    update_statistics: S,
) -> io::Result<()>
where
    W: FnMut(String) + Clone + Send + 'static,
    C: FnMut(String, Color) + Clone + Send + 'static,
    S: FnMut(String) + Clone + Send + 'static,
{
    let stream = TcpStream::connect(&ip).await?;

    let mut client = Client {
        stream,
        status: 0,
        compression: -1,
    };

    let state = State {
        shutdown: false,
        my_entity_id: None,
        my_pos: (0.0, 0.0, 0.0),
        mobs: HashMap::new(),
        hotbar: [Slot::NotPresent; 9],
        sneaking: false,
        held_item: settings.lock().await.weapon,
        food: 20.0,
        initial_statistics: Vec::new(),
        statistics: Vec::new(),
    };

    client
        .send(ServerBound::Handshake(
            VarInt(753),
            MString("bruh".to_string()),
            25565,
            VarInt(2),
        ))
        .await?;

    client
        .send(ServerBound::LoginStart(MString(username)))
        .await?;

    let client = Arc::new(Mutex::new(client));
    let state = Arc::new(Mutex::new(state));

    async fn check<W: FnMut(String) + Clone + Send + 'static>(
        mut write_to_log: W,
        client: Arc<Mutex<Client>>,
        state: Arc<Mutex<State>>,
        shutdown_sender: mpsc::Sender<()>,
        res: io::Result<()>,
    ) {
        if let Err(e) = res {
            if state.lock().await.shutdown {
                if e.kind() != ErrorKind::BrokenPipe && e.kind() != ErrorKind::UnexpectedEof {
                    write_to_log(format!("Error: {}", e));
                }
            } else {
                write_to_log(format!("Error: {}", e));
                state.lock().await.shutdown = true;
                client
                    .lock()
                    .await
                    .stream
                    .shutdown(std::net::Shutdown::Both)
                    .unwrap();
            }
            shutdown_sender.send(()).await.unwrap();
        }
    }

    // spawn a task to start/stop sneaking depending on the settings in real time
    // and change held item
    let task1 = tokio::spawn({
        clone_all![client, state, settings, write_to_log, shutdown_sender];
        async move {
            check(
                write_to_log,
                client.clone(),
                state.clone(),
                shutdown_sender,
                async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs_f32(0.1)).await;
                        let mut state_lock = state.lock().await;
                        let settings_lock = settings.lock().await;
                        // change hotbar item if needed
                        if state_lock.held_item != settings_lock.weapon {
                            client
                                .lock()
                                .await
                                .send(ServerBound::HeldItemChange(settings_lock.weapon as i16))
                                .await?;
                            state_lock.held_item = settings_lock.weapon;
                        }

                        // sneaking things
                        if let Some(my_entity_id) = state_lock.my_entity_id {
                            let settings_sneaking = settings_lock.sneak;
                            let current_sneaking = state_lock.sneaking;
                            if settings_sneaking == current_sneaking {
                                continue;
                            }
                            if settings_sneaking {
                                // start sneaking
                                client
                                    .lock()
                                    .await
                                    .send(ServerBound::EntityAction(
                                        VarInt(my_entity_id.0),
                                        VarInt(0),
                                        VarInt(0),
                                    ))
                                    .await?;
                            } else {
                                // stop sneaking
                                client
                                    .lock()
                                    .await
                                    .send(ServerBound::EntityAction(
                                        VarInt(my_entity_id.0),
                                        VarInt(1),
                                        VarInt(1),
                                    ))
                                    .await?;
                            }
                            state_lock.sneaking = settings_sneaking;
                        }
                    }
                }
                .await,
            )
            .await;
        }
    });

    // spawn a task for querying the statistics
    let task2 = tokio::spawn({
        clone_all![client, state, write_to_log, shutdown_sender];
        async move {
            check(
                write_to_log,
                client.clone(),
                state.clone(),
                shutdown_sender,
                async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs_f32(1.0)).await;
                        // request statistics
                        client
                            .lock()
                            .await
                            .send(ServerBound::ClientStatus(VarInt(1)))
                            .await?;
                    }
                }
                .await,
            )
            .await;
        }
    });

    // also spawn a task for eating when needed
    let task3 = tokio::spawn({
        clone_mut![write_to_log];
        clone_all![client, state, settings, shutdown_sender];
        async move {
            // the compiler desires this to be put into a variable for some reason
            let write_to_log_clone = write_to_log.clone();
            check(
                write_to_log_clone,
                client.clone(),
                state.clone(),
                shutdown_sender,
                async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs_f32(1.0)).await;
                        let state_lock = state.lock().await;
                        let settings_lock = settings.lock().await;

                        if state_lock.food < settings_lock.eat_at {
                            // gotta eat something
                            // find food in hotbar
                            for slot_id in 0..9 {
                                if let Slot::Present(id, _number) = &state_lock.hotbar[slot_id] {
                                    if settings_lock.eat_food.contains(&id.0) {
                                        write_to_log(format!("Eating."));
                                        // its eatable and allowed to eat
                                        // eat it
                                        client
                                            .lock()
                                            .await
                                            .send(ServerBound::HeldItemChange(slot_id as i16))
                                            .await?;
                                        client
                                            .lock()
                                            .await
                                            .send(ServerBound::UseItem(VarInt(0)))
                                            .await?;
                                        // wait 1.63 s and then finish eating
                                        tokio::time::sleep(std::time::Duration::from_secs_f32(
                                            1.61,
                                        ))
                                        .await;
                                        client
                                            .lock()
                                            .await
                                            .send(ServerBound::PlayerDigging(VarInt(5), 0, 0))
                                            .await?;
                                        client
                                            .lock()
                                            .await
                                            .send(ServerBound::HeldItemChange(
                                                settings_lock.weapon as i16,
                                            ))
                                            .await?;
                                        // stop searching for food this iteration
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                .await,
            )
            .await;
        }
    });

    // spawn a task for attacking nearby mobs every once in a while
    let task4 = tokio::spawn({
        clone_all![client, state, settings, write_to_log, shutdown_sender];
        async move {
            check(
                write_to_log,
                client.clone(),
                state.clone(),
                shutdown_sender,
                async move {
                    let mut i = 0;
                    loop {
                        let attack_speed = settings.lock().await.attack_speed;
                        let to_sleep = if attack_speed == 0.0 {
                            0.5
                        } else {
                            1.0 / attack_speed
                        };
                        tokio::time::sleep(std::time::Duration::from_secs_f32(to_sleep)).await;
                        if attack_speed == 0.0 {
                            // skip the attack
                            continue;
                        }
                        // calculate the nearest mob
                        let state_lock = state.lock().await;
                        if state_lock.mobs.len() > 0 {
                            let sq_dist = |mob: &Mob| {
                                let my_pos = state_lock.my_pos;

                                (my_pos.0 - mob.pos.0).powi(2)
                                    + (my_pos.1 - mob.pos.1).powi(2)
                                    + (my_pos.2 - mob.pos.2).powi(2)
                            };

                            let mut nearest_mob_id = None;
                            let mut nearest_mob_squared_distance = None;
                            let settings_lock = settings.lock().await;
                            for (id, mob) in state_lock.mobs.iter() {
                                if settings_lock.attack_mobs.contains(&mob.entity_type.0) {
                                    let dist = sq_dist(mob);
                                    if let Some(temp_nearest_mob_squared_distance) =
                                        nearest_mob_squared_distance
                                    {
                                        if dist < temp_nearest_mob_squared_distance {
                                            nearest_mob_id = Some(id.clone());
                                            nearest_mob_squared_distance = Some(dist);
                                        }
                                    } else {
                                        nearest_mob_id = Some(id.clone());
                                        nearest_mob_squared_distance = Some(dist);
                                    }
                                }
                            }
                            drop(settings_lock);
                            // if a mob was found, attack it
                            if let (Some(nearest_mob_id), Some(nearest_mob_squared_distance)) =
                                (nearest_mob_id, nearest_mob_squared_distance)
                            {
                                if nearest_mob_squared_distance >= 16.0 {
                                    continue;
                                }

                                // change the player rotation to look at the mob
                                // this is not neccessary but helps when debugging
                                // because you can see what mob the bot is trying to attack
                                //
                                // calculate pitch and yaw
                                let mob = &state_lock.mobs[&nearest_mob_id];
                                let dx = mob.pos.0 - state_lock.my_pos.0;
                                let dy = mob.pos.1 - state_lock.my_pos.1;
                                let dz = mob.pos.2 - state_lock.my_pos.2;
                                let r = (dx * dx + dy * dy + dz * dz).sqrt();
                                let mut yaw = -dx.atan2(dz) / std::f64::consts::PI * 180.0;
                                if yaw < 0.0 {
                                    yaw = 360.0 + yaw;
                                }
                                let pitch = -(dy / r).asin() / std::f64::consts::PI * 180.0;
                                client
                                    .lock()
                                    .await
                                    .send(ServerBound::PlayerPositionAndRotation(
                                        state_lock.my_pos.0,
                                        state_lock.my_pos.1 - 1.62,
                                        state_lock.my_pos.2,
                                        yaw as f32,
                                        pitch as f32,
                                        true,
                                    ))
                                    .await?;

                                // attack the mob
                                client
                                    .lock()
                                    .await
                                    .send(ServerBound::InteractEntity(
                                        nearest_mob_id,
                                        VarInt(1),
                                        settings.lock().await.sneak,
                                    ))
                                    .await?;
                                // also animation
                                client
                                    .lock()
                                    .await
                                    .send(ServerBound::Animation(VarInt(i)))
                                    .await?;
                                i = (i + 1) % 2;
                            }
                        }
                    }
                }
                .await,
            )
            .await;
        }
    });

    // spawn a task for processing incoming packets
    let task5 = tokio::spawn({
        clone_all![
            client,
            state,
            settings,
            write_to_log,
            shutdown_sender,
            update_statistics
        ];
        async move {
            // the compiler desires this to be put into a variable for some reason
            let write_to_log_clone = write_to_log.clone();
            check(
                write_to_log_clone,
                client.clone(),
                state.clone(),
                shutdown_sender.clone(),
                async move {
                    loop {
                        let packet = client.lock().await.receive().await?;

                        // spawn a task for proccessing this packet
                        tokio::spawn({
                            clone_mut![change_status, write_to_log, update_statistics];
                            clone_all![client, state, settings, shutdown_sender];
                            // pls rustc
                            let write_to_log_clone = write_to_log.clone();
                            async move {
                                check(
                                    write_to_log_clone,
                                    client.clone(),
                                    state.clone(),
                                    shutdown_sender.clone(),
                                    async move {
                                        match packet {
                                            ClientBound::KeepAlive(id) => {
                                                // send the keepalive packet back
                                                client
                                                    .lock()
                                                    .await
                                                    .send(ServerBound::KeepAlive(id))
                                                    .await?;
                                            }
                                            ClientBound::JoinGame(my_entity_id) => {
                                                state.lock().await.my_entity_id =
                                                    Some(VarInt(my_entity_id as i64));
                                                client
                                                    .lock()
                                                    .await
                                                    .send(ServerBound::HeldItemChange(
                                                        settings.lock().await.weapon as i16,
                                                    ))
                                                    .await?;
                                            }
                                            ClientBound::UpdateHealth(health, food, _) => {
                                                change_status(
                                                    format!(
                                                        "health: {}/20   food: {}/20",
                                                        health, food.0
                                                    ),
                                                    Color::DarkGreen,
                                                );
                                                state.lock().await.food = food.0 as f32;
                                                let min_health = settings.lock().await.min_health;
                                                if (health as f32) < min_health {
                                                    write_to_log(format!("Health ({}) below {}, disconnecting to avoid death.", health, min_health));

                                                    shutdown_sender.send(()).await.unwrap();
                                                }
                                            }
                                            ClientBound::PlayerPositionAndLook(
                                                x,
                                                y,
                                                z,
                                                _yaw,
                                                _pitch,
                                                _,
                                                id,
                                            ) => {
                                                state.lock().await.my_pos = (x, y, z);
                                                client
                                                    .lock()
                                                    .await
                                                    .send(ServerBound::TeleportConfirm(id))
                                                    .await?;
                                            }
                                            ClientBound::SpawnLivingEntity(
                                                entity_id,
                                                _,
                                                entity_type,
                                                x,
                                                y,
                                                z,
                                                ..,
                                            ) => {
                                                // add the mob to the list of mobs
                                                state.lock().await.mobs.insert(
                                                    entity_id.clone(),
                                                    Mob {
                                                        entity_id,
                                                        entity_type,
                                                        pos: (x, y, z),
                                                    },
                                                );
                                            }
                                            ClientBound::SetSlot(window_id, slot_id, slot_data) => {
                                                if window_id == 0 && slot_id >= 36 && slot_id <= 44
                                                {
                                                    // hotbar item changed
                                                    state.lock().await.hotbar
                                                        [slot_id as usize - 36] = slot_data;
                                                }
                                            }
                                            ClientBound::DestroyEntities(ids) => {
                                                // remove mobs
                                                let mut state_lock = state.lock().await;
                                                for id in ids {
                                                    state_lock.mobs.remove(&id);
                                                }
                                            }
                                            ClientBound::EntityTeleport(entity_id, x, y, z, ..) => {
                                                // change mob position
                                                if let Some(mob) =
                                                    state.lock().await.mobs.get_mut(&entity_id)
                                                {
                                                    mob.pos = (x, y, z);
                                                }
                                            }
                                            ClientBound::EntityPosition(
                                                entity_id,
                                                delta_x,
                                                delta_y,
                                                delta_z,
                                                _,
                                            ) => {
                                                // change mob position
                                                // calculate the new position
                                                if let Some(mob) =
                                                    state.lock().await.mobs.get_mut(&entity_id)
                                                {
                                                    let new_pos = |old, delta| {
                                                        ((delta as f64) / 128.0 + old * 32.0) / 32.0
                                                    };
                                                    mob.pos = (
                                                        new_pos(mob.pos.0, delta_x),
                                                        new_pos(mob.pos.1, delta_y),
                                                        new_pos(mob.pos.2, delta_z),
                                                    );
                                                }
                                            }
                                            ClientBound::PlayDisconnect(reason) => {
                                                return Err(Error::new(
                                                    ErrorKind::Other,
                                                    format!("Kicked: {:?}", reason),
                                                ))
                                            }
                                            ClientBound::LoginDisconnect(reason) => {
                                                return Err(Error::new(
                                                    ErrorKind::Other,
                                                    format!("Kicked: {:?}", reason),
                                                ))
                                            }
                                            ClientBound::Statistics(statistics) => {
                                                let mut state_lock = state.lock().await;
                                                if state_lock.initial_statistics.len() == 0 {
                                                    state_lock.initial_statistics = statistics.clone();
                                                    state_lock.statistics = statistics;
                                                    update_statistics(format!("No statistics to show yet."));
                                                } else {
                                                    let mut statistics_str = String::new();
                                                    for statistic in statistics {
                                                        // we're only interested in the "killed" category
                                                        if (statistic.0).0 != 6 {
                                                            continue;
                                                        }
                                                        // get the old value
                                                        let mut old = state_lock.statistics.iter_mut().find(|e| (e.0).0 == (statistic.0).0 && (e.1).0 == (statistic.1).0).unwrap();
                                                        old.2 = statistic.2;
                                                    }
                                                    // update the gui
                                                    for statistic in &state_lock.statistics {
                                                        // again, we're only interested in the "killed" category
                                                        if (statistic.0).0 != 6 {
                                                            continue;
                                                        }
                                                        // get the initial value
                                                        let initial = state_lock.initial_statistics.iter().find(|e| (e.0).0 == (statistic.0).0 && (e.1).0 == (statistic.1).0).unwrap();
                                                        // calculate the difference
                                                        let difference = (statistic.2).0 - (initial.2).0;
                                                        if difference == 0 {
                                                            continue;
                                                        }
                                                        statistics_str += &format!("{}: {}\n", MOBS.get_by_left(&(statistic.1).0).unwrap_or(&"{UNKNOWN MOB}"), difference);
                                                    }
                                                    if statistics_str.len() == 0 {
                                                        update_statistics(format!("No statistics to show yet."));
                                                    } else {
                                                        update_statistics(statistics_str);
                                                    }
                                                }
                                            },
                                            ClientBound::Unknown(_) => {}
                                            _other => {
                                                // println!("Received {:?}", other);
                                            }
                                        }
                                        Ok(())
                                    }
                                    .await,
                                )
                                .await;
                            }
                        });
                    }
                }
                .await,
            )
            .await;
        }
    });

    shutdown_receiver.recv().await;
    task1.abort();
    task2.abort();
    task3.abort();
    task4.abort();
    task5.abort();

    Ok(())
}
