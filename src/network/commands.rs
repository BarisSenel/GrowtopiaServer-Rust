use crate::network::host::Host;
use crate::database::player::Player;
use crate::network::packet::{GamePacket, VariantListBuilder};
use tracing::info;

pub fn handle_command(
    host: &mut Host,
    peer_id: u32,
    player: &Player,
    command_text: &str,
    peer_worlds: &mut std::collections::HashMap<u32, String>,
    peer_names: &mut std::collections::HashMap<u32, String>,
    peer_pos: &mut std::collections::HashMap<u32, (f32, f32)>,
    peer_hidden_players: &mut std::collections::HashSet<u32>,
) {
    let parts: Vec<&str> = command_text.split_whitespace().collect();
    if parts.is_empty() { return; }

    let cmd = parts[0].to_lowercase();
    let args = &parts[1..];

    info!("Peer {} ({}) executing command: /{} with args {:?}", peer_id, player.name, cmd, args);

    match cmd.as_str() {
        "help" | "?" => {
            send_console_msg(host, peer_id, "`wAvailable Commands: ``/help, /stats, /hideplayers, /showplayers, /nick <name>");
        }
        "stats" => {
            let stats_msg = format!("`wStats for {}: ``Level: `w{}``, XP: `w{}``, Gems: `w{}``",
                player.name, player.level, player.xp, player.gems);
            send_console_msg(host, peer_id, &stats_msg);
        }
        "status" => {
            let equipped_str = player.equipped.iter().enumerate()
                .map(|(i, &id)| format!("{}:{}", i, id))
                .collect::<Vec<String>>()
                .join(", ");

            let slots_str = player.slots.iter()
                .map(|s| format!("{}:{}", s.item_id, s.count))
                .collect::<Vec<String>>()
                .join(", ");

            send_console_msg(host, peer_id, &format!("`wEquipped (Idx:ID): ``[{}]", equipped_str));
            send_console_msg(host, peer_id, &format!("`wInventory: ``[{}]", slots_str));
        }
        "hideplayers" => {
            peer_hidden_players.insert(peer_id);


            let mut remove = GamePacket::new();
            remove.packet_type = 1;

            if let Some(world_name) = peer_worlds.get(&peer_id) {
                for (&other_peer, other_world) in peer_worlds.iter() {
                    if other_peer == peer_id || other_world != world_name {
                        continue;
                    }
                    let (rem_data, rem_count) = VariantListBuilder::new()
                        .add_string("OnRemove")
                        .add_string(&format!("netID|{}\n", other_peer))
                        .add_string(&format!("pId|1\n"))
                        .build();
                    host.send(peer_id, &remove.to_bytes(&rem_data, rem_count), 0).ok();
                }
            }
            send_console_msg(host, peer_id, "Other players are now `4hidden``. Type `w/showplayers`` to see them again.");
        }
        "showplayers" => {
            peer_hidden_players.remove(&peer_id);


            if let Some(world_name) = peer_worlds.get(&peer_id) {
                let mut spawn = GamePacket::new();
                spawn.packet_type = 1;

                for (&other_peer, other_world) in peer_worlds.iter() {
                    if other_peer == peer_id || other_world != world_name {
                        continue;
                    }
                    let other_name = peer_names.get(&other_peer).cloned().unwrap_or_else(|| "Unknown".to_string());
                    let (ox, oy) = peer_pos.get(&other_peer).cloned().unwrap_or((1000.0, 1000.0));

                    let other_spawn_text = format!(
                        "spawn|avatar\nnetID|{}\nuserID|{}\ncolrect|0|0|20|30\nposXY|{}|{}\nname|`w{}``\ncountry|tr\ninvis|0\nmstate|0\nsmstate|0\nonlineID|\n",
                        other_peer, 1, ox, oy, other_name
                    );
                    let (os_data, os_count) = VariantListBuilder::new()
                        .add_string("OnSpawn")
                        .add_string(&other_spawn_text)
                        .build();
                    host.send(peer_id, &spawn.to_bytes(&os_data, os_count), 0).ok();
                }
            }
            send_console_msg(host, peer_id, "Other players are now `2visible``.");
        }
        "nick" => {
            if args.is_empty() {
                send_console_msg(host, peer_id, "`4Usage: ``/nick <new_name>");
            } else {
                let new_nick = args.join(" ");
                send_console_msg(host, peer_id, &format!("`wNicks are currently disabled. Requested: {}``", new_nick));
            }
        }
        "roles" => {
            let menu = crate::game::gui::build_role_menu(player, "roleTab_human", peer_id as i32);
            let (d_data, d_c) = VariantListBuilder::new()
                .add_string("OnDialogRequest").add_string(&menu).build();
            let mut packet = GamePacket::new();
            packet.packet_type = 1;
            host.send(peer_id, &packet.to_bytes(&d_data, d_c), 0).ok();
        }
        "farmer" => {
            let menu = crate::game::gui::build_farmer_menu(player, peer_id as i32);
            let (d_data, d_c) = VariantListBuilder::new()
                .add_string("OnDialogRequest").add_string(&menu).build();
            let mut packet = GamePacket::new();
            packet.packet_type = 1;
            host.send(peer_id, &packet.to_bytes(&d_data, d_c), 0).ok();
        }
        _ => {
            send_console_msg(host, peer_id, &format!("`4Unknown command: ``/{} (type `/help` for list)", cmd));
        }
    }
}

fn send_console_msg(host: &mut Host, peer_id: u32, message: &str) {
    let mut msg = GamePacket::new();
    msg.packet_type = 1;
    let (data, count) = VariantListBuilder::new()
        .add_string("OnConsoleMessage")
        .add_string(message)
        .build();
    host.send(peer_id, &msg.to_bytes(&data, count), 0).ok();
}