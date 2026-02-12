use crate::network::host::{Host, HostEvent};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{info, error, warn, debug};
use std::fs;
use std::thread;
use std::sync::mpsc::{Receiver, Sender};
use crate::database::{player, world};
use crate::database::db_thread::DbCommand;
use base64::{engine::general_purpose, Engine as _};
use std::collections::{HashMap, HashSet};

const MAX_PEERS: u32 = 50;
const CHANNEL_LIMIT: u8 = 2;

pub enum ServerCommand {
    GiveItem { player_name: String, item_id: i32, amount: i32 },
    SetLevel { player_name: String, level: i32 },
    AddXP { player_name: String, xp: i32 },
    SpawnBoss { world_name: String, health: i32 },
}



fn send_packet(host: &mut Host, peer_id: u32, packet: Vec<u8>) {
    host.send(peer_id, &packet, 0).ok();
}

fn send_variant(host: &mut Host, peer_id: u32, data: Vec<u8>, count: u8, net_id: i32, delay: i32) {
    let mut pkt = crate::network::packet::GamePacket::new();
    pkt.packet_type = 1;
    pkt.net_id = net_id;
    pkt.id = delay;
    let bytes = pkt.to_bytes(&data, count);
    send_packet(host, peer_id, bytes);
}

fn send_console_message(host: &mut Host, peer_id: u32, message: &str) {
    let (data, count) = crate::network::packet::VariantListBuilder::new()
        .add_string("OnConsoleMessage")
        .add_string(message)
        .build();
    send_variant(host, peer_id, data, count, -1, 0);
}

fn broadcast_to_world(
    host: &mut Host,
    peer_worlds: &HashMap<u32, String>,
    peer_hidden: &HashSet<u32>,
    target_world: &str,
    packet_data: &[u8],
    exclude_peer: Option<u32>,
    check_hidden: bool,
) {
    for (&p_id, w_name) in peer_worlds.iter() {
        if w_name == target_world {
            if let Some(ex) = exclude_peer {
                if ex == p_id { continue; }
            }
            if check_hidden && peer_hidden.contains(&p_id) {
                continue;
            }
            send_packet(host, p_id, packet_data.to_vec());
        }
    }
}

fn find_peer_by_name(peer_names: &HashMap<u32, String>, name: &str) -> Option<u32> {
    for (&id, p_name) in peer_names {
        if p_name.to_lowercase() == name.to_lowercase() {
            return Some(id);
        }
    }
    None
}

fn format_spawn_avatar(net_id: u32, user_id: i32, x: f32, y: f32, name: &str, is_local: bool) -> String {
    format!(
        "spawn|avatar\nnetID|{}\nuserID|{}\ncolrect|0|0|20|30\nposXY|{}|{}\nname|`w{}``\ncountry|tr\ninvis|0\nmstate|0\nsmstate|0\nonlineID|\n{}",
        net_id, user_id, x, y, name, if is_local { "type|local\n" } else { "" }
    )
}

fn broadcast_on_remove(host: &mut Host, peer_id: u32, peer_worlds: &HashMap<u32, String>) {
    if let Some(world_name) = peer_worlds.get(&peer_id) {
        let (data, count) = crate::network::packet::VariantListBuilder::new()
            .add_string("OnRemove")
            .add_string(&format!("netID|{}\n", peer_id))
            .add_string("pId|1\n")
            .build();
        let mut pkt = crate::network::packet::GamePacket::new();
        pkt.packet_type = 1;
        let bytes = pkt.to_bytes(&data, count);

        for (&p_id, w_name) in peer_worlds.iter() {
            if p_id != peer_id && w_name == world_name {
                send_packet(host, p_id, bytes.clone());
            }
        }
    }
}



fn send_inventory(host: &mut Host, peer_id: u32, player: &player::Player) {
    let mut inv_pkt = crate::network::packet::GamePacket::new();
    inv_pkt.packet_type = 0x09;
    inv_pkt.net_id = peer_id as i32;
    inv_pkt.peer_state = 0x08;

    let mut inv_data = Vec::new();
    use bytes::BufMut;
    inv_data.put_u16(16);
    inv_data.put_u32(player.slots.len() as u32);

    for slot in &player.slots {
        let val = (slot.item_id as u32 & 0xFFFF) | ((slot.count as u32 & 0xFF) << 16);
        inv_data.put_u32_le(val);
    }

    send_packet(host, peer_id, inv_pkt.to_bytes_with_raw_data(&inv_data));
}

fn send_world_select_menu(host: &mut Host, peer_id: u32) {
    let menu_text = "add_filter|\nadd_heading|Top Worlds<ROW2>|\nadd_floater|wotd_world|\u{013B} WOTD|0|0.5|3529161471\nadd_heading|My Worlds<CR>|\nadd_heading|Recently Visited Worlds<CR>|\n";
    let (data, count) = crate::network::packet::VariantListBuilder::new()
        .add_string("OnRequestWorldSelectMenu").add_string(menu_text).add_int(1).build();
    send_variant(host, peer_id, data, count, -1, 0);
}

pub fn send_on_set_clothing(host: &mut Host, target_peer: u32, owner_peer_id: u32, player: &player::Player, delay: i32) {
    let get_equip = |i| player.equipped.get(i).cloned().unwrap_or(0) as f32;
    let (data, count) = crate::network::packet::VariantListBuilder::new()
        .add_string("OnSetClothing")
        .add_vec3(get_equip(0), get_equip(1), get_equip(2))
        .add_vec3(get_equip(3), get_equip(4), get_equip(5))
        .add_vec3(get_equip(6), get_equip(7), get_equip(8))
        .add_int(player.skin_color as i32)
        .add_vec3(get_equip(9), 0.0, 0.0)
        .build();
    send_variant(host, target_peer, data, count, owner_peer_id as i32, delay);
}

fn calculate_punch_id(player: &player::Player) -> u8 {

    if let Some(&item_id) = player.equipped.get(5) {
        if item_id != 0 {
            let config = crate::database::item_config::get_item_config(item_id);
            if config.visual_effect != 0 {
                return config.visual_effect;
            }
        }
    }

    for &item_id in &player.equipped {
        if item_id != 0 {
            let config = crate::database::item_config::get_item_config(item_id);
            if config.visual_effect != 0 {
                return config.visual_effect;
            }
        }
    }
    0
}

fn trigger_punch_effects(
    host: &mut Host,
    peer_worlds: &HashMap<u32, String>,
    peer_hidden: &HashSet<u32>,
    world_name: &str,
    peer_id: u32,
    item_id: i32,
    pos_x: f32,
    pos_y: f32,
) {
    let config = crate::database::item_config::get_item_config(item_id);
    let effects = config.get_effects();


    if let Some(particle_id) = effects.particle_id {
        let mut pkt = crate::network::packet::GamePacket::new();
        pkt.packet_type = 17;
        pkt.net_id = peer_id as i32;
        pkt.pos_x = pos_x;
        pkt.pos_y = pos_y;
        pkt.speed_y = particle_id as f32;

        broadcast_to_world(host, peer_worlds, peer_hidden, world_name, &pkt.to_bytes(&[], 0), None, false);
    }


    if let Some(audio_path) = effects.audio_path {
        let (data, count) = crate::network::packet::VariantListBuilder::new()
            .add_string("OnPlayPositioned")
            .add_string(&audio_path)
            .build();

        let mut pkt = crate::network::packet::GamePacket::new();
        pkt.packet_type = 1;
        pkt.net_id = peer_id as i32;

        broadcast_to_world(host, peer_worlds, peer_hidden, world_name, &pkt.to_bytes(&data, count), None, false);
    }
}

pub fn broadcast_on_set_clothing(
    host: &mut Host,
    peer_worlds: &HashMap<u32, String>,
    peer_pos: &HashMap<u32, (f32, f32)>,
    peer_hidden: &HashSet<u32>,
    world_name: &str,
    owner_peer_id: u32,
    player: &player::Player,
) {
    let (ox, oy) = peer_pos.get(&owner_peer_id).cloned().unwrap_or((1000.0, 1000.0));

    let pupil = 0x000000FF;
    let hair = 0xFFFFFFFF;
    let eyes = 0xFFFFFFFF;

    let punch_id = calculate_punch_id(player);

    for (&target_peer, world) in peer_worlds.iter() {
        if world == world_name {
            if peer_hidden.contains(&target_peer) { continue; }
            send_on_set_clothing(host, target_peer, owner_peer_id, player, 0);
            send_set_character_state(host, target_peer, owner_peer_id as i32, ox, oy, pupil, hair, eyes, punch_id);
        }
    }
}

fn send_role_skins(_host: &mut Host, _target_peer: u32, _owner_peer_id: u32) {

}



fn send_set_character_state(
    host: &mut Host,
    target_peer: u32,
    owner_net_id: i32,
    x: f32,
    y: f32,
    pupil_color: u32,
    hair_color: u32,
    eye_color: u32,
    punch_id: u8,
) {
    let mut pkt = crate::network::packet::GamePacket::new();
    pkt.packet_type = (0x14) | ((punch_id as i32) << 8) | (0x80 << 16) | (0x80 << 24);
    pkt.net_id = owner_net_id;
    pkt.peer_state = 0;
    pkt.pos_x = x; pkt.pos_y = y;
    pkt.speed_x = 250.0; pkt.speed_y = 1000.0; pkt.count = 125.0;


    pkt.uid = pupil_color as i32;
    pkt.punch_x = hair_color as i32;
    pkt.punch_y = eye_color as i32;

    send_packet(host, target_peer, pkt.to_bytes(&[], 0));
}



pub fn start_enet_server(cmd_rx: Receiver<ServerCommand>, db_tx: Sender<DbCommand>) -> Result<(), String> {
    let gs_port: u16 = std::env::var("gameserver_port")
        .unwrap_or_else(|_| "17091".to_string())
        .parse()
        .unwrap_or(17091);
    let gs_ip = std::env::var("gameserver_adress").unwrap_or_else(|_| "127.0.0.1".to_string());
    let gs_token = std::env::var("GAMESERVER_TOKEN").unwrap_or_else(|_| "0260DCEB9063AC540552C15E90E9E639".to_string());

    let start_items = std::time::Instant::now();
    let items_dat = fs::read("items.dat").unwrap_or_else(|_| {
        error!("items.dat not found in root directory!");
        Vec::new()
    });
    info!("Loaded items.dat ({} bytes) in {:.2?}", items_dat.len(), start_items.elapsed());

    let mut host = Host::new(
        "0.0.0.0", gs_port, MAX_PEERS, CHANNEL_LIMIT,
        false, true, None, None, true, true,
    )?;

    info!("ENet Server listening on 0.0.0.0:{}", gs_port);


    let mut peer_worlds: HashMap<u32, String> = HashMap::new();
    let mut peer_names: HashMap<u32, String> = HashMap::new();
    let mut peer_players: HashMap<u32, player::Player> = HashMap::new();
    let mut peer_pos: HashMap<u32, (f32, f32)> = HashMap::new();
    let mut peer_hidden_players: HashSet<u32> = HashSet::new();
    let mut active_worlds: HashMap<String, world::World> = HashMap::new();

    loop {

        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                ServerCommand::GiveItem { player_name, item_id, amount } => {
                    if let Some(p_id) = find_peer_by_name(&peer_names, &player_name) {
                        if let Some(player) = peer_players.get_mut(&p_id) {
                            if let Some(slot) = player.slots.iter_mut().find(|s| s.item_id == item_id) {
                                slot.count += amount;
                            } else {
                                player.slots.push(player::InventorySlot { item_id, count: amount });
                            }
                            db_tx.send(DbCommand::UpdatePlayer(player.clone())).ok();
                            send_console_message(&mut host, p_id, &format!("`wAdmin`` gave you `w{}`` of item `w{}``!", amount, item_id));
                            send_inventory(&mut host, p_id, player);
                            info!("Gave {} x {} to {}", item_id, amount, player.name);
                        }
                    } else { info!("Player {} not found online", player_name); }
                }
                ServerCommand::SetLevel { player_name, level } => {
                    if let Some(p_id) = find_peer_by_name(&peer_names, &player_name) {
                        if let Some(player) = peer_players.get_mut(&p_id) {
                            player.level = level;
                            db_tx.send(DbCommand::UpdatePlayer(player.clone())).ok();
                            send_console_message(&mut host, p_id, &format!("`wAdmin`` set your level to `w{}``!", level));
                            info!("Set level of {} to {}", player.name, level);
                        }
                    } else { info!("Player {} not found online", player_name); }
                }
                ServerCommand::AddXP { player_name, xp } => {
                    if let Some(p_id) = find_peer_by_name(&peer_names, &player_name) {
                        if let Some(player) = peer_players.get_mut(&p_id) {
                            player.xp += xp;
                            db_tx.send(DbCommand::UpdatePlayer(player.clone())).ok();
                            send_console_message(&mut host, p_id, &format!("`wAdmin`` gave you `w{}`` XP!", xp));
                            info!("Gave {} XP to {}", xp, player.name);
                        }
                    } else { info!("Player {} not found online", player_name); }
                }
                ServerCommand::SpawnBoss { world_name, health } => {
                    let world_upper = world_name.to_uppercase();


                    if let Some(world) = active_worlds.get_mut(&world_upper) {

                         let mut net_id = 1000;
                         while world.npcs.iter().any(|n| n.net_id == net_id) {
                             net_id += 1;
                         }

                         let npc = crate::game::npc::Npc::new(
                             net_id,
                             format!("Boss `4({}/{})``", health, health),
                             (world.width as f32 * 32.0) / 2.0,
                             (world.height as f32 * 32.0) / 2.0 - 100.0,
                             health
                         );


                         let spawn_packet = format_spawn_avatar(
                             npc.net_id,
                             npc.net_id as i32,
                             npc.x,
                             npc.y,
                             &npc.name,
                             false
                         );


                         let (v_data, v_count) = crate::network::packet::VariantListBuilder::new()
                            .add_string("OnSpawn")
                            .add_string(&spawn_packet)
                            .build();

                         let mut pkt = crate::network::packet::GamePacket::new();
                         pkt.packet_type = 1;
                         pkt.net_id = -1;
                         pkt.id = -1;
                         let bytes = pkt.to_bytes(&v_data, v_count);

                         broadcast_to_world(&mut host, &peer_worlds, &peer_hidden_players, &world_upper, &bytes, None, false);

                         info!("Spawned NPC {} (ID: {}) in {}", npc.name, npc.net_id, world_upper);
                         world.npcs.push(npc);

                    } else {
                        info!("World {} is not active (no players?). Cannot spawn boss.", world_upper);

                    }
                }
            }
        }

        match host.service() {
            Ok(Some(event)) => match event {
                HostEvent::Connect { peer_id } => {
                    info!("Peer connected: {}", peer_id);
                    send_packet(&mut host, peer_id, vec![1, 0, 0, 0]);
                }
                HostEvent::Receive { peer_id, channel_id: _, data } => {
                    if data.len() >= 4 {
                        let packet_type = data[0];
                        match packet_type {
                            2 | 3 => {
                                let header = String::from_utf8_lossy(&data[4..]).trim_matches('\0').to_string();
                                let pipes_str = header.replace('\n', "|");
                                let pipes: Vec<&str> = pipes_str.split('|').filter(|s| !s.is_empty()).collect();

                                let mut data_map = HashMap::new();
                                for i in (0..pipes.len()).step_by(2) {
                                    if i + 1 < pipes.len() {
                                        data_map.insert(pipes[i].trim().to_string(), pipes[i+1].trim().to_string());
                                    }
                                }


                                let mut resolved_password = data_map.get("password").cloned();
                                let mut resolved_name = None;
                                let mut resolved_discord_id = None;

                                if let Some(ltoken_b64) = data_map.get("ltoken").or_else(|| data_map.get("LTOKEN")) {
                                    if let Ok(decoded_bytes) = general_purpose::STANDARD.decode(ltoken_b64) {
                                        let decoded_str = String::from_utf8_lossy(&decoded_bytes);
                                        for part in decoded_str.split('&') {
                                            if let Some(val) = part.strip_prefix("password=") { resolved_password = Some(val.to_string()); }
                                            else if let Some(val) = part.strip_prefix("growId=") { resolved_name = Some(val.to_string()); }
                                            else if let Some(val) = part.strip_prefix("_token=") { resolved_discord_id = Some(val.to_string()); }
                                        }
                                    }
                                }

                                if let Some(ref discord_id) = resolved_discord_id {
                                    if let Ok(Some(mut player)) = player::get_player_by_discord_id(discord_id) {

                                        if let Some(ref pwd) = resolved_password {
                                            if player.ltoken.as_ref() != Some(pwd) {
                                                player.ltoken = Some(pwd.clone());
                                                db_tx.send(DbCommand::UpdatePlayer(player.clone())).ok();
                                            }
                                        }
                                        peer_names.insert(peer_id, player.name.clone());
                                    } else if let Some(ref name) = resolved_name {
                                        let mut new_player = player::Player::new(name);
                                        new_player.discord_id = Some(discord_id.clone());
                                        new_player.ltoken = resolved_password.clone();
                                        db_tx.send(DbCommand::UpdatePlayer(new_player.clone())).ok();
                                        peer_names.insert(peer_id, name.clone());
                                    }
                                } else if let Some(ref password) = resolved_password {
                                    if let Ok(Some(player)) = player::get_player_by_ltoken(password) {
                                        peer_names.insert(peer_id, player.name.clone());
                                    }
                                }


                                if !pipes.is_empty() {
                                    let mut action = data_map.get("action").cloned()
                                        .or_else(|| pipes.get(0).map(|s| s.to_string()))
                                        .unwrap_or_default();

                                    if action.starts_with("action|") {
                                        action = action.strip_prefix("action|").unwrap().to_string();
                                    }

                                    if action == "protocol" {
                                        let name = peer_names.get(&peer_id).cloned()
                                            .or_else(|| data_map.get("tankIDName").cloned())
                                            .or_else(|| data_map.get("requestedName").cloned())
                                            .unwrap_or_else(|| "GrowtopiaUser".to_string());

                                        peer_names.insert(peer_id, name.clone());


                                        let (id_data, id_count) = crate::network::packet::VariantListBuilder::new()
                                            .add_string("SetHasGrowID").add_int(1).add_string(&name).add_string("").build();
                                        send_variant(&mut host, peer_id, id_data, id_count, -1, 0);


                                        let redirect_str = format!("{}|0|{}", gs_ip, gs_token);
                                        let (red_data, red_count) = crate::network::packet::VariantListBuilder::new()
                                            .add_string("OnSendToServer").add_int(gs_port as i32).add_int(8172597).add_int(12345)
                                            .add_string(&redirect_str).add_int(1).add_string(&name).build();
                                        send_variant(&mut host, peer_id, red_data, red_count, -1, 0);

                                        host.disconnect_later_peer(peer_id, 0).ok();

                                    } else if action == "tankIDName" {
                                        let player_name = data_map.get("tankIDName").cloned()
                                            .or_else(|| data_map.get("requestedName").cloned())
                                            .or_else(|| peer_names.get(&peer_id).cloned())
                                            .unwrap_or_else(|| "Unknown".to_string());


                                        if let Some(old_id) = find_peer_by_name(&peer_names, &player_name) {
                                            if old_id != peer_id {
                                                warn!("Kicking duplicate session {} (Peer {})", player_name, old_id);
                                                broadcast_on_remove(&mut host, old_id, &peer_worlds);
                                                send_console_message(&mut host, old_id, "`4Logged in from another location.``");
                                                host.disconnect_now_peer(old_id, 0).ok();

                                                peer_names.remove(&old_id);
                                                peer_players.remove(&old_id);
                                                peer_worlds.remove(&old_id);
                                                peer_pos.remove(&old_id);
                                            }
                                        }

                                        peer_names.insert(peer_id, player_name.clone());

                                        let mut current_player = match player::load_player(&player_name) {
                                            Ok(Some(p)) => p,
                                            _ => {
                                                let p = player::Player::new(&player_name);
                                                db_tx.send(DbCommand::UpdatePlayer(p.clone())).ok();
                                                p
                                            }
                                        };


                                        let mut changed = false;
                                        if !current_player.slots.iter().any(|s| s.item_id == 18) {
                                            current_player.slots.push(player::InventorySlot { item_id: 18, count: 1 });
                                            changed = true;
                                        }
                                        if !current_player.slots.iter().any(|s| s.item_id == 32) {
                                            current_player.slots.push(player::InventorySlot { item_id: 32, count: 1 });
                                            changed = true;
                                        }
                                        if changed { db_tx.send(DbCommand::UpdatePlayer(current_player.clone())).ok(); }

                                        peer_players.insert(peer_id, current_player.clone());
                                        info!("Player {} logged in.", current_player.name);



                                        let (ftue, c) = crate::network::packet::VariantListBuilder::new()
                                            .add_string("OnFtueButtonDataSet").add_int(0).add_int(0).add_int(0)
                                            .add_string("||0|||-1").add_string("").add_string("1|1").build();
                                        send_variant(&mut host, peer_id, ftue, c, -1, 0);


                                        let (hid, c) = crate::network::packet::VariantListBuilder::new()
                                            .add_string("SetHasGrowID").add_int(1).add_string(&current_player.name).add_string("").build();
                                        send_variant(&mut host, peer_id, hid, c, -1, 0);


                                        let (gdpr, c) = crate::network::packet::VariantListBuilder::new()
                                            .add_string("OnOverrideGDPRFromServer").add_int(18).add_int(1).add_int(0).add_int(1).build();
                                        send_variant(&mut host, peer_id, gdpr, c, -1, 0);


                                        let (skin, c) = crate::network::packet::VariantListBuilder::new()
                                            .add_string("OnSetRoleSkinsAndTitles").add_string("000000").add_string("000000").build();
                                        send_variant(&mut host, peer_id, skin, c, -1, 0);


                                        let (logon, c) = crate::network::packet::VariantListBuilder::new()
                                            .add_string("OnSuperMainStartAcceptLogonHrdxs47254722215a")
                                            .add_uint(2816436900)
                                            .add_string(&std::env::var("webserver_adress").unwrap_or("chaosautomations.com".to_string()))
                                            .add_string("cache/")
                                            .add_string("cc.cz.madkite.freedom org.aqua.gg idv.aqua.bulldog com.cih.gamecih2 com.cih.gamecih com.cih.game_cih cn.maocai.gamekiller com.gmd.speedtime org.dax.attack com.x0.strai.frep com.x0.strai.free org.cheatengine.cegui org.sbtools.gamehack com.skgames.traffikrider org.sbtoods.gamehaca com.skype.ralder org.cheatengine.cegui.xx.multi1458919170111 com.prohiro.macro me.autotouch.autotouch com.cygery.repetitouch.free com.cygery.repetitouch.pro com.proziro.zacro com.slash.gamebuster")
                                            .add_string("proto=225|choosemusic=audio/mp3/about_theme.mp3|active_holiday=0|wing_week_day=0|ubi_week_day=0|server_tick=33784663|game_theme=|clash_active=1|drop_lavacheck_faster=1|isPayingUser=1|usingStoreNavigation=1|enableInventoryTab=1|bigBackpack=1|seed_diary_hash=3701384193|m_clientBits=|eventButtons={\"EventButtonData\":[{\"active\":true,\"buttonAction\":\"eventmenu\",\"buttonState\":0,\"buttonTemplate\":\"BaseEventButton\",\"counter\":0,\"counterMax\":0,\"itemIdIcon\":6828,\"name\":\"ClashEventButton\",\"notification\":0,\"order\":9,\"rcssClass\":\"clash-event\",\"text\":\"Nah\"}]}")
                                            .build();
                                        send_variant(&mut host, peer_id, logon, c, -1, 0);


                                        let (eb, c) = crate::network::packet::VariantListBuilder::new()
                                            .add_string("OnEventButtonDataSet").add_string("ClashEventButton").add_int(1)
                                            .add_string("{\"active\":true,\"buttonAction\":\"eventmenu\",\"buttonState\":0,\"buttonTemplate\":\"BaseEventButton\",\"counter\":0,\"counterMax\":0,\"itemIdIcon\":6828,\"name\":\"ClashEventButton\",\"notification\":0,\"order\":9,\"rcssClass\":\"clash-event\",\"text\":\"Claim!\"}")
                                            .build();
                                        send_variant(&mut host, peer_id, eb, c, -1, 0);

                                    } else if action == "refresh_item_data" {
                                        send_console_message(&mut host, peer_id, "One moment, updating item data...");
                                        let mut items_pkt = crate::network::packet::GamePacket::new();
                                        items_pkt.packet_type = 0x10;
                                        items_pkt.peer_state = 0x08;
                                        send_packet(&mut host, peer_id, items_pkt.to_bytes_with_raw_data(&items_dat));

                                    } else if action == "wrench" {

                                        let target_net_id = data_map.get("netid").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);

                                        if target_net_id == peer_id {

                                            if let Some(player) = peer_players.get(&peer_id) {
                                                let menu = crate::game::gui::build_profile_menu(player, peer_id as i32);
                                                let (d_data, d_c) = crate::network::packet::VariantListBuilder::new()
                                                    .add_string("OnDialogRequest").add_string(&menu).build();
                                                send_variant(&mut host, peer_id, d_data, d_c, -1, 0);
                                            }
                                        } else {

                                            let target_name = peer_names.get(&target_net_id).cloned().unwrap_or("Unknown".to_string());
                                            let dialog = format!(
                                                "set_default_color|`o\nadd_label_with_icon|big|`w{}``|left|18|\nadd_spacer|small|\nadd_textbox|This is a player.|left|\nend_dialog|profile|OK||\n",
                                                target_name
                                            );
                                            let (d_data, d_c) = crate::network::packet::VariantListBuilder::new()
                                                .add_string("OnDialogRequest").add_string(&dialog).build();
                                            send_variant(&mut host, peer_id, d_data, d_c, -1, 0);
                                        }

                                        } else if action == "setSkin" {
                                            if let Some(color_str) = data_map.get("color") {
                                                if let Ok(color) = color_str.parse::<u32>() {
                                                    if let Some(player) = peer_players.get_mut(&peer_id) {
                                                        player.skin_color = color;
                                                        db_tx.send(DbCommand::UpdatePlayer(player.clone())).ok();


                                                        if let Some(w_name) = peer_worlds.get(&peer_id) {
                                                            broadcast_on_set_clothing(&mut host, &peer_worlds, &peer_pos, &peer_hidden_players, w_name, peer_id, player);
                                                        }
                                                    }
                                                }
                                            }
                                        } else if action == "dialog_return" {
                                        let dialog_name = data_map.get("dialog_name").cloned().unwrap_or_default();
                                        if dialog_name == "role_menu" {
                                            if let Some(button_clicked) = data_map.get("buttonClicked") {
                                                if button_clicked.starts_with("roleTab_") {

                                                     if let Some(player) = peer_players.get(&peer_id) {
                                                        let menu = crate::game::gui::build_role_menu(player, button_clicked, peer_id as i32);
                                                        let (d_data, d_c) = crate::network::packet::VariantListBuilder::new()
                                                            .add_string("OnDialogRequest").add_string(&menu).build();
                                                        send_variant(&mut host, peer_id, d_data, d_c, -1, 0);
                                                    }
                                                }
                                            }
                                        } else if dialog_name == "popup" {
                                            if let Some(button_clicked) = data_map.get("buttonClicked") {
                                                if button_clicked == "goals" {
                                                    if let Some(player) = peer_players.get(&peer_id) {
                                                        let menu = crate::game::gui::build_milestones_menu(player, peer_id as i32);
                                                        let (d_data, d_c) = crate::network::packet::VariantListBuilder::new()
                                                            .add_string("OnDialogRequest").add_string(&menu).build();
                                                        send_variant(&mut host, peer_id, d_data, d_c, -1, 0);
                                                    }
                                                }
                                            }
                                        } else if dialog_name == "setSkin" {

                                            if let Some(color_str) = data_map.get("color") {
                                                if let Ok(color) = color_str.parse::<u32>() {
                                                    if let Some(player) = peer_players.get_mut(&peer_id) {
                                                        player.skin_color = color;
                                                        db_tx.send(DbCommand::UpdatePlayer(player.clone())).ok();
                                                        if let Some(w_name) = peer_worlds.get(&peer_id) {
                                                            broadcast_on_set_clothing(&mut host, &peer_worlds, &peer_pos, &peer_hidden_players, w_name, peer_id, player);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else if action == "enter_game" {
                                        if !peer_worlds.contains_key(&peer_id) {
                                            let name = peer_names.get(&peer_id).cloned().unwrap_or_else(|| "Unknown".to_string());


                                            let (ftue, c) = crate::network::packet::VariantListBuilder::new()
                                                .add_string("OnFtueButtonDataSet").add_int(0).add_int(0).add_int(0)
                                                .add_string("||0|||-1").add_string("").add_string("1|1").build();
                                            send_variant(&mut host, peer_id, ftue, c, -1, 0);


                                            send_console_message(&mut host, peer_id, &format!("Welcome back, `w{}````.", name));


                                            let (date, c) = crate::network::packet::VariantListBuilder::new()
                                                .add_string("OnTodaysDate").add_int(2).add_int(3).add_int(0).add_int(0).build();
                                            send_variant(&mut host, peer_id, date, c, -1, 0);


                                            send_world_select_menu(&mut host, peer_id);


                                            let gazette_text = "add_spacer|small|\nadd_label_with_icon|big|`wThe Growtopia Rust Server``|left|5016|\nadd_spacer|small|\nadd_textbox|`wFebruary 3rd: `5First Build``|left|\nadd_spacer|small|\nadd_textbox|Welcome to the new Rust server! |left|\nadd_quick_exit|\nend_dialog|gazette||OK|";
                                            let (gaz, c) = crate::network::packet::VariantListBuilder::new()
                                                .add_string("OnDialogRequest").add_string(gazette_text).build();
                                            send_variant(&mut host, peer_id, gaz, c, -1, 0);


                                            let mut ping = crate::network::packet::GamePacket::new();
                                            ping.packet_type = 0x16;
                                            ping.peer_state = 0x08;
                                            send_packet(&mut host, peer_id, ping.to_bytes(&[], 0));
                                        }

                                    } else if action == "join_request" {
                                        let world_name = data_map.get("name").cloned().unwrap_or_else(|| "START".to_string()).to_uppercase();
                                        let player_obj = match peer_players.get(&peer_id) {
                                            Some(p) => p.clone(),
                                            None => { host.disconnect_later_peer(peer_id, 0).ok(); continue; }
                                        };


                                        if let Some(old_world) = peer_worlds.get(&peer_id).cloned() {
                                            if old_world != world_name {
                                                broadcast_on_remove(&mut host, peer_id, &peer_worlds);
                                            }
                                        }
                                        peer_worlds.insert(peer_id, world_name.clone());


                                        let current_world = if let Some(w) = active_worlds.get(&world_name) {
                                            w.clone()
                                        } else {
                                            match world::load_world(&world_name) {
                                                Ok(Some(w)) => { active_worlds.insert(world_name.clone(), w.clone()); w },
                                                _ => {
                                                    let nw = world::World::new(&world_name);
                                                    db_tx.send(DbCommand::UpdateWorld(nw.clone())).ok();
                                                    active_worlds.insert(world_name.clone(), nw.clone());
                                                    nw
                                                }
                                            }
                                        };


                                        if !current_world.owner_name.is_empty() {
                                            send_console_message(&mut host, peer_id, &format!("[`2World Locked by {}`3]", current_world.owner_name));
                                        }


                                        let mut map_pkt = crate::network::packet::GamePacket::new();
                                        map_pkt.packet_type = 4;
                                        map_pkt.peer_state = 8;
                                        send_packet(&mut host, peer_id, map_pkt.to_bytes_with_raw_data(&current_world.to_bytes()));


                                        let mut spawn_x = 1000.0;
                                        let mut spawn_y = 1000.0;
                                        for (i, tile) in current_world.tiles.iter().enumerate() {
                                            if tile.fg == 6 {
                                                spawn_x = ((i as u32 % current_world.width) * 32) as f32;
                                                spawn_y = ((i as u32 / current_world.width) * 32) as f32;
                                                break;
                                            }
                                        }
                                        peer_pos.insert(peer_id, (spawn_x, spawn_y));


                                        let local_spawn = format_spawn_avatar(peer_id, peer_id as i32, spawn_x, spawn_y, &player_obj.name, true);
                                        let (ls_data, ls_c) = crate::network::packet::VariantListBuilder::new()
                                            .add_string("OnSpawn").add_string(&local_spawn).build();
                                        send_variant(&mut host, peer_id, ls_data, ls_c, -1, -1);


                                        let (ox, oy) = (spawn_x, spawn_y);
                                        let p_id = calculate_punch_id(&player_obj);
                                        send_on_set_clothing(&mut host, peer_id, peer_id, &player_obj, 100);
                                        send_set_character_state(&mut host, peer_id, peer_id as i32, ox, oy, 0x000000FF, 0xFFFFFFFF, 0xFFFFFFFF, p_id);



                                        let mut other_count = 0;
                                        for (&other_peer, other_world) in peer_worlds.iter() {
                                            if other_peer == peer_id || *other_world != world_name { continue; }
                                            other_count += 1;

                                            let other_p = peer_players.get(&other_peer).cloned().unwrap_or_else(|| player::Player::new("Unknown"));
                                            let (ox, oy) = peer_pos.get(&other_peer).cloned().unwrap_or((spawn_x, spawn_y));


                                            let ex_spawn = format_spawn_avatar(other_peer, other_peer as i32, ox, oy, &other_p.name, false);
                                            let (ex_data, ex_c) = crate::network::packet::VariantListBuilder::new()
                                                .add_string("OnSpawn").add_string(&ex_spawn).build();
                                            send_variant(&mut host, peer_id, ex_data, ex_c, -1, -1);

                                            let o_p_id = calculate_punch_id(&other_p);
                                            send_on_set_clothing(&mut host, peer_id, other_peer, &other_p, 100);
                                            send_set_character_state(&mut host, peer_id, other_peer as i32, ox, oy, 0x000000FF, 0xFFFFFFFF, 0xFFFFFFFF, o_p_id);



                                            let join_spawn = format_spawn_avatar(peer_id, peer_id as i32, spawn_x, spawn_y, &player_obj.name, false);
                                            let (js_data, js_c) = crate::network::packet::VariantListBuilder::new()
                                                .add_string("OnSpawn").add_string(&join_spawn).build();
                                            send_variant(&mut host, other_peer, js_data, js_c, -1, -1);

                                            let p_id = calculate_punch_id(&player_obj);
                                            send_on_set_clothing(&mut host, other_peer, peer_id, &player_obj, 100);
                                            send_set_character_state(&mut host, other_peer, peer_id as i32, spawn_x, spawn_y, 0x000000FF, 0xFFFFFFFF, 0xFFFFFFFF, p_id);

                                        }

                                        send_inventory(&mut host, peer_id, &player_obj);

                                        send_console_message(&mut host, peer_id, &format!("World `w{}`` entered. `w{}`` others here.", world_name, other_count));

                                    } else if action == "quit_to_exit" || action == "quit" {
                                        if peer_worlds.contains_key(&peer_id) {

                                            broadcast_on_remove(&mut host, peer_id, &peer_worlds);
                                            peer_worlds.remove(&peer_id);
                                            send_world_select_menu(&mut host, peer_id);
                                        }

                                    } else if action == "input" {
                                        if let Some(text) = data_map.get("text") {
                                            if text.starts_with('/') {
                                                if let Some(player) = peer_players.get(&peer_id) {
                                                    crate::network::commands::handle_command(
                                                        &mut host, peer_id, player, &text[1..],
                                                        &mut peer_worlds, &mut peer_names, &mut peer_pos, &mut peer_hidden_players
                                                    );
                                                }
                                            } else {
                                                if let (Some(w_name), Some(name)) = (peer_worlds.get(&peer_id), peer_names.get(&peer_id)) {
                                                    let talk_bubble = format!("CP:0_PL:0_OID:_player_chat={}", text);
                                                    let console = format!("<`w{}``> {}", name, text);

                                                    let mut pkt = crate::network::packet::GamePacket::new();
                                                    pkt.packet_type = 1;

                                                    let (b_data, b_c) = crate::network::packet::VariantListBuilder::new()
                                                        .add_string("OnTalkBubble").add_int(peer_id as i32).add_string(&talk_bubble).build();
                                                    let b_bytes = pkt.to_bytes(&b_data, b_c);

                                                    let (c_data, c_c) = crate::network::packet::VariantListBuilder::new()
                                                        .add_string("OnConsoleMessage").add_string(&console).build();
                                                    let c_bytes = pkt.to_bytes(&c_data, c_c);

                                                    broadcast_to_world(&mut host, &peer_worlds, &peer_hidden_players, w_name, &b_bytes, None, false);
                                                    broadcast_to_world(&mut host, &peer_worlds, &peer_hidden_players, w_name, &c_bytes, None, false);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            4 | 10 => {
                                if let Some((mut packet, _)) = crate::network::packet::GamePacket::from_bytes(&data) {

                                    let is_interaction = packet.packet_type == 3 && packet.punch_x != -1;

                                    if packet.packet_type == 10 {
                                        let item_id = packet.id;
                                        if let Some(slot_idx) = crate::database::item_config::get_clothing_type(item_id) {
                                            let has_item = peer_players.get(&peer_id).map_or(false, |p| p.slots.iter().any(|s| s.item_id == item_id));

                                            if has_item {
                                                if let Some(player) = peer_players.get_mut(&peer_id) {
                                                    let s_idx = slot_idx as usize;
                                                    if player.equipped.len() <= s_idx { player.equipped.resize(s_idx + 1, 0); }


                                                    if player.equipped[s_idx] == item_id { player.equipped[s_idx] = 0; }
                                                    else { player.equipped[s_idx] = item_id; }


                                                    db_tx.send(DbCommand::UpdatePlayer(player.clone())).ok();


                                                    if let Some(w_name) = peer_worlds.get(&peer_id) {
                                                        broadcast_on_set_clothing(&mut host, &peer_worlds, &peer_pos, &peer_hidden_players, w_name, peer_id, player);
                                                    }

                                                    let (en, c) = crate::network::packet::VariantListBuilder::new()
                                                        .add_string("OnEquipNewItem").add_int(item_id).build();
                                                    send_variant(&mut host, peer_id, en, c, -1, 0);


                                                }
                                            }
                                        }
                                    } else if packet.packet_type == 0 {
                                        peer_pos.insert(peer_id, (packet.pos_x, packet.pos_y));
                                        if let Some(w_name) = peer_worlds.get(&peer_id) {
                                            packet.net_id = peer_id as i32;
                                            broadcast_to_world(&mut host, &peer_worlds, &peer_hidden_players, w_name, &packet.to_bytes(&[], 0), Some(peer_id), true);


                                            if let Some(world) = active_worlds.get_mut(w_name) {
                                                let p_x = packet.pos_x;
                                                let p_y = packet.pos_y;


                                                for npc in world.npcs.iter_mut() {
                                                    if npc.health > 0 {
                                                        let dx = (npc.x - p_x).abs();
                                                        let dy = (npc.y - p_y).abs();




                                                        if dx < 50.0 && dy < 50.0 {



                                                            if (packet.peer_state & 256) != 0 || (packet.peer_state & 2048) != 0 || (packet.peer_state & 0x4000) != 0 {
                                                                 println!("HIT NPC {}! Flags: {}", npc.net_id, packet.peer_state);
                                                                 npc.health -= 5;
                                                                 npc.name = format!("Boss `4({}/{})``", npc.health, npc.max_health);


                                                                 let msg = format!("action|setNpcName\nnetID|{}\nname|{}", npc.net_id, npc.name);




                                                                 let (v_data, v_count) = crate::network::packet::VariantListBuilder::new()
                                                                    .add_string("OnNameChanged")
                                                                    .add_string(&npc.name)
                                                                    .build();


                                                                 let mut name_pkt = crate::network::packet::GamePacket::new();
                                                                 name_pkt.packet_type = 1;
                                                                 name_pkt.net_id = npc.net_id as i32;
                                                                 let nb = name_pkt.to_bytes(&v_data, v_count);
                                                                 broadcast_to_world(&mut host, &peer_worlds, &peer_hidden_players, w_name, &nb, None, false);


                                                                 let mut visual = crate::network::packet::GamePacket::new();
                                                                 visual.packet_type = 8;
                                                                 visual.net_id = npc.net_id as i32;
                                                                 visual.pos_x = npc.x; visual.pos_y = npc.y;
                                                                 visual.count = 5.0;
                                                                 visual.id = 6;
                                                                 broadcast_to_world(&mut host, &peer_worlds, &peer_hidden_players, w_name, &visual.to_bytes(&[], 0), None, false);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else if is_interaction {
                                        if let Some(world_name) = peer_worlds.get(&peer_id).cloned() {
                                            if let Some(current_world) = active_worlds.get_mut(&world_name) {
                                                if packet.id == 0 { packet.id = 18; }


                                                if let Some((px, py)) = peer_pos.get(&peer_id) {
                                                    trigger_punch_effects(&mut host, &peer_worlds, &peer_hidden_players, &world_name, peer_id, packet.id, *px, *py);
                                                }


                                                let mut has_item = true;
                                                if packet.id != 18 && packet.id != 32 && packet.id != 6 && packet.id != 8 {
                                                     if let Some(player) = peer_players.get(&peer_id) {
                                                         has_item = player.slots.iter().any(|s| s.item_id == packet.id && s.count > 0);
                                                     }
                                                }

                                                if !has_item {


                                                    continue;
                                                }

                                                let p_name = peer_names.get(&peer_id).cloned().unwrap_or("Unk".into());
                                                packet.packet_type = 3;
                                                let res = current_world.handle_tile_change(&mut packet, &p_name);


                                                if res != world::TileChangeResult::NoChange {
                                                    db_tx.send(DbCommand::UpdateWorld(current_world.clone())).ok();


                                                    match res {
                                                        world::TileChangeResult::PlacedFG(id) | world::TileChangeResult::PlacedBG(id) => {
                                                            if let Some(player) = peer_players.get_mut(&peer_id) {
                                                                if let Some(slot) = player.slots.iter_mut().find(|s| s.item_id == id as i32) {
                                                                    if slot.count > 0 {
                                                                        slot.count -= 1;
                                                                        if slot.count == 0 {


                                                                        }
                                                                        db_tx.send(DbCommand::UpdatePlayer(player.clone())).ok();
                                                                        send_inventory(&mut host, peer_id, player);
                                                                    }
                                                                }
                                                            }
                                                        },
                                                        _ => {}
                                                    }
                                                }

                                                match res {
                                                    world::TileChangeResult::Damaged(_, hits) => {
                                                        let mut visual = packet.clone();
                                                        visual.packet_type = 8;
                                                        visual.id = 6; visual.count = hits as f32; visual.net_id = peer_id as i32;
                                                        broadcast_to_world(&mut host, &peer_worlds, &peer_hidden_players, &world_name, &visual.to_bytes(&[], 0), None, false);
                                                    },
                                                    world::TileChangeResult::BrokeFG(block_id, hits) | world::TileChangeResult::BrokeBG(block_id, hits) => {
                                                        let mut visual = packet.clone();
                                                        visual.packet_type = 8;
                                                        visual.id = 6; visual.count = hits as f32; visual.net_id = peer_id as i32;
                                                        broadcast_to_world(&mut host, &peer_worlds, &peer_hidden_players, &world_name, &visual.to_bytes(&[], 0), None, false);

                                                        packet.net_id = peer_id as i32;
                                                        broadcast_to_world(&mut host, &peer_worlds, &peer_hidden_players, &world_name, &packet.to_bytes(&[], 0), None, false);


                                                        if let Some(player) = peer_players.get_mut(&peer_id) {
                                                            let (xp, levelled_up) = crate::game::gt_mmo::check_farmer_xp(player, block_id as u32);
                                                            if xp > 0 {
                                                                db_tx.send(DbCommand::UpdatePlayer(player.clone())).ok();


                                                                if levelled_up {













                                                                    let mut particle = crate::network::packet::GamePacket::new();
                                                                    particle.packet_type = 0x11;
                                                                    particle.net_id = peer_id as i32;
                                                                    particle.pos_x = player.equipped.len() as f32;

                                                                    if let Some((px, py)) = peer_pos.get(&peer_id) {
                                                                        particle.pos_x = *px;
                                                                        particle.pos_y = *py;
                                                                    }
                                                                    particle.speed_y = 46.0;


                                                                    broadcast_to_world(&mut host, &peer_worlds, &peer_hidden_players, &world_name, &particle.to_bytes(&[], 0), None, false);



                                                                    let msg = format!("`2{}`` reached farming level {}!", player.name, player.farmer_lvl);

                                                                    let mut pkt = crate::network::packet::GamePacket::new();
                                                                    pkt.packet_type = 1;
                                                                    let (b_data, b_c) = crate::network::packet::VariantListBuilder::new()
                                                                        .add_string("OnTalkBubble").add_int(peer_id as i32).add_string(&msg).build();

                                                                    broadcast_to_world(&mut host, &peer_worlds, &peer_hidden_players, &world_name, &pkt.to_bytes(&b_data, b_c), None, false);

                                                                    send_console_message(&mut host, peer_id, &msg);
                                                                }
                                                            }
                                                        }
                                                    },
                                                    world::TileChangeResult::PlacedFG(_) | world::TileChangeResult::PlacedBG(_) => {
                                                        packet.net_id = peer_id as i32;
                                                        broadcast_to_world(&mut host, &peer_worlds, &peer_hidden_players, &world_name, &packet.to_bytes(&[], 0), None, false);
                                                    },
                                                    _ => {

                                                        let i = (packet.punch_y * (current_world.width as i32) + packet.punch_x) as usize;
                                                        if i < current_world.tiles.len() {
                                                            let t = &current_world.tiles[i];
                                                            packet.id = if t.fg != 0 { t.fg as i32 } else { t.bg as i32 };
                                                        }
                                                        packet.net_id = peer_id as i32;
                                                        send_packet(&mut host, peer_id, packet.to_bytes(&[], 0));
                                                    }
                                                }



                                                match res {
                                                    world::TileChangeResult::Damaged(block_id, _) |
                                                    world::TileChangeResult::BrokeFG(block_id, _) |
                                                    world::TileChangeResult::BrokeBG(block_id, _) => {

                                                        let equipped_items = peer_players.get(&peer_id).map(|p| p.equipped.as_slice()).unwrap_or(&[]);


                                                        let is_left = (packet.peer_state & 0x10) != 0;


                                                        let effects = crate::game::item_effects::handle_punch_effects(&mut *current_world, packet.punch_x, packet.punch_y, is_left, equipped_items, block_id as i32);


                                                        for &(ex, ey, eres) in &effects {
                                                            match eres {
                                                                world::TileChangeResult::Damaged(_, ehits) => {
                                                                    let mut visual = packet.clone();
                                                                    visual.packet_type = 8;
                                                                    visual.punch_x = ex; visual.punch_y = ey;
                                                                    visual.id = 6; visual.count = ehits as f32; visual.net_id = peer_id as i32;
                                                                    broadcast_to_world(&mut host, &peer_worlds, &peer_hidden_players, &world_name, &visual.to_bytes(&[], 0), None, false);
                                                                },
                                                                world::TileChangeResult::BrokeFG(eid, ehits) | world::TileChangeResult::BrokeBG(eid, ehits) => {

                                                                    if let Some(player) = peer_players.get_mut(&peer_id) {
                                                                        let (xp, leveled_up) = crate::game::gt_mmo::check_farmer_xp(player, eid as u32);
                                                                        if xp > 0 {
                                                                            if leveled_up {

                                                                                let (p_data, p_c) = crate::network::packet::VariantListBuilder::new()
                                                                                    .add_string("OnParticleEffect").add_int(0x05).add_vec2(ex as f32 * 32.0, ey as f32 * 32.0).add_int(0).add_int(0).build();
                                                                                send_variant(&mut host, peer_id, p_data, p_c, -1, 0);

                                                                                let bubble = format!("`5Level Up! `oYou are now Level `2{} `oFarmer!", player.farmer_lvl);
                                                                                let (b_data, b_c) = crate::network::packet::VariantListBuilder::new()
                                                                                    .add_string("OnTalkBubble").add_int(peer_id as i32).add_string(&bubble).add_int(0).build();

                                                                                let mut v_pkt = crate::network::packet::GamePacket::new();
                                                                                v_pkt.packet_type = 1;
                                                                                broadcast_to_world(&mut host, &peer_worlds, &peer_hidden_players, &world_name, &v_pkt.to_bytes(&b_data, b_c), None, false);


                                                                                send_console_message(&mut host, peer_id, &format!("`5{} `oearned `2{} `oXP! (Level `2{} `o/ `2{} `oXP)",
                                                                                    crate::game::gt_mmo::get_milestone_title(player.farmer_lvl as u32).unwrap_or("Farmer"),
                                                                                    xp, player.farmer_lvl, player.farmer_xp));
                                                                            }

                                                                            db_tx.send(DbCommand::UpdatePlayer(player.clone())).ok();
                                                                        }
                                                                    }


                                                                    let mut visual = packet.clone();
                                                                    visual.packet_type = 8;
                                                                    visual.punch_x = ex; visual.punch_y = ey;
                                                                    visual.id = 6; visual.count = ehits as f32; visual.net_id = peer_id as i32;
                                                                    broadcast_to_world(&mut host, &peer_worlds, &peer_hidden_players, &world_name, &visual.to_bytes(&[], 0), None, false);


                                                                    let mut change_pkt = packet.clone();
                                                                    change_pkt.packet_type = 3;
                                                                    change_pkt.punch_x = ex; change_pkt.punch_y = ey;
                                                                    change_pkt.net_id = peer_id as i32;
                                                                    broadcast_to_world(&mut host, &peer_worlds, &peer_hidden_players, &world_name, &change_pkt.to_bytes(&[], 0), None, false);
                                                                },
                                                                _ => {}
                                                            }
                                                        }

                                                        if !effects.is_empty() {

                                                            db_tx.send(DbCommand::UpdateWorld(current_world.clone())).ok();
                                                        }
                                                    }
                                                    _ => {}
                                                }


                                            }
                                        }
                                    } else if packet.packet_type == 7 {
                                        if let Some(world_name) = peer_worlds.get(&peer_id).cloned() {
                                            if let Some(current_world) = active_worlds.get(&world_name) {

                                                let i = (packet.punch_y * (current_world.width as i32) + packet.punch_x) as usize;
                                                if i < current_world.tiles.len() && current_world.tiles[i].fg == 6 {

                                                    if peer_worlds.contains_key(&peer_id) {
                                                        broadcast_on_remove(&mut host, peer_id, &peer_worlds);
                                                        peer_worlds.remove(&peer_id);
                                                        send_world_select_menu(&mut host, peer_id);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                HostEvent::Disconnect { peer_id, .. } => {
                    info!("Peer disconnected: {}", peer_id);

                    if let Some(player) = peer_players.get(&peer_id) {

                        db_tx.send(DbCommand::UpdatePlayer(player.clone())).ok();
                    }
                    broadcast_on_remove(&mut host, peer_id, &peer_worlds);
                    peer_worlds.remove(&peer_id);
                    peer_names.remove(&peer_id);
                    peer_players.remove(&peer_id);
                    peer_pos.remove(&peer_id);
                    peer_hidden_players.remove(&peer_id);
                }
            },
            Ok(None) => {
                thread::sleep(Duration::from_millis(1));
            }
            Err(e) => {
                error!("ENet Error: {}", e);
            }
        }


        let now = std::time::SystemTime::now();

        for (w_name, world) in active_worlds.iter_mut() {
             let users_in_world: Vec<u32> = peer_worlds.iter().filter(|&(_, w)| w == w_name).map(|(p, _)| *p).collect();
             if users_in_world.is_empty() { continue; }



             let mut dead_npcs = Vec::new();

             for npc in world.npcs.iter_mut() {

                     if npc.health <= 0 {

                         dead_npcs.push(npc.net_id);


                         use rand::Rng;
                         let mut rng = rand::thread_rng();

                         for _ in 0..2 {
                            let off_x = rng.gen_range(-10.0..=10.0);
                            let off_y = rng.gen_range(-10.0..=10.0);


                            let mut eff_pkt = crate::network::packet::GamePacket::new();
                            eff_pkt.packet_type = 17;
                            eff_pkt.net_id = -1;
                            eff_pkt.pos_x = npc.x + 16.0 + off_x;
                            eff_pkt.pos_y = npc.y + 16.0 + off_y;
                            eff_pkt.speed_y = 90.0;

                            let eff_bytes = eff_pkt.to_bytes(&[], 0);

                            for &p_id in &users_in_world {
                                send_packet(&mut host, p_id, eff_bytes.clone());
                            }
                         }


                         for &p_id in &users_in_world {
                             send_console_message(&mut host, p_id, "`4BOSS DEFEATED!``");
                         }

                         continue;
                     }

                 let mut needs_update = false;


                 if let Ok(elapsed) = now.duration_since(npc.last_jump) {
                     if elapsed.as_millis() > 2000 {
                         npc.last_jump = now;
                         use rand::Rng;
                         let mut rng = rand::thread_rng();
                         let move_dir = rng.gen_range(0..4);
                         match move_dir {
                             0 => npc.x -= 32.0,
                             1 => npc.x += 32.0,


                             _ => {}
                         }

                         npc.x = npc.x.clamp(0.0, world.width as f32 * 32.0);
                         npc.y = npc.y.clamp(0.0, world.height as f32 * 32.0);
                         needs_update = true;
                     }
                 }

                 if needs_update {
                      let mut pkt = crate::network::packet::GamePacket::new();
                      pkt.packet_type = 0;
                      pkt.net_id = npc.net_id as i32;
                      pkt.pos_x = npc.x;
                      pkt.pos_y = npc.y;
                      pkt.pos_y = npc.y;
                      pkt.peer_state = 0;

                      let data = pkt.to_bytes(&[], 0);
                      for &p_id in &users_in_world {
                          send_packet(&mut host, p_id, data.clone());
                      }
                 }
             }


             if !dead_npcs.is_empty() {
                 world.npcs.retain(|n| !dead_npcs.contains(&n.net_id));
                 for nid in dead_npcs {
                     let msg = format!("netID|{}\n", nid);
                     let pid_str = format!("pId|{}\n", nid);


                     let (v_data, v_count) = crate::network::packet::VariantListBuilder::new()
                        .add_string("OnRemove")
                        .add_string(&msg)
                        .add_string(&pid_str)
                        .build();



                     let mut rem_pkt = crate::network::packet::GamePacket::new();
                     rem_pkt.packet_type = 1;
                     rem_pkt.net_id = -1;
                     let rem_bytes = rem_pkt.to_bytes(&v_data, v_count);

                     for &p_id in &users_in_world {

                         send_packet(&mut host, p_id, rem_bytes.clone());
                     }
                 }
             }

        }
    }
}