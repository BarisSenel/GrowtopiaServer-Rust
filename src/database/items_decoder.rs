use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use tracing::{info, error};
use super::item_config::ItemConfig;

pub fn load_item_definitions() -> HashMap<i32, ItemConfig> {
    let mut m = HashMap::new();
    let data = match std::fs::read("items.dat") {
        Ok(d) => d,
        Err(e) => {
            error!("Failed to load items.dat: {}. Using default hardcoded items.", e);


             m.insert(6, ItemConfig { id: 6, clothing_type: 0, action_type: 0, hits_to_break: 255, is_breakable: false, is_background: false, name: "Main Door".to_string(), punch_effect: None, visual_effect: 0, rayman: 0, punch_options: String::new() });
             m.insert(8, ItemConfig { id: 8, clothing_type: 0, action_type: 0, hits_to_break: 255, is_breakable: false, is_background: false, name: "Bedrock".to_string(), punch_effect: None, visual_effect: 0, rayman: 0, punch_options: String::new() });
            return m;
        }
    };

    let mut pos = 0;


    let read_u16 = |data: &[u8], pos: &mut usize| -> u16 {
        if *pos + 2 > data.len() { return 0; }
        let val = u16::from_le_bytes([data[*pos], data[*pos+1]]);
        *pos += 2;
        val
    };
    let read_u32 = |data: &[u8], pos: &mut usize| -> u32 {
        if *pos + 4 > data.len() { return 0; }
        let val = u32::from_le_bytes([data[*pos], data[*pos+1], data[*pos+2], data[*pos+3]]);
        *pos += 4;
        val
    };
    let read_str = |data: &[u8], pos: &mut usize, item_id: Option<i32>| -> String {
        let len = read_u16(data, pos) as usize;
        if *pos + len > data.len() { return String::new(); }
        let mut bytes = data[*pos..*pos+len].to_vec();
        *pos += len;

        if let Some(id) = item_id {
            let key = "PBG892FXX982ABC*";
            for (i, byte) in bytes.iter_mut().enumerate() {
                *byte ^= key.as_bytes()[(id as usize + i) % key.len()];
            }
        }
        String::from_utf8(bytes).unwrap_or_default()
    };

    let version = read_u16(&data, &mut pos);
    let item_count = read_u32(&data, &mut pos);

    info!("Loading items.dat: Version {}, Item Count {}", version, item_count);

    for _ in 0..item_count {
        let item_id = read_u32(&data, &mut pos) as i32;

        let _editable_type = data[pos]; pos += 1;
        let _item_category = data[pos]; pos += 1;
        let action_type = data[pos]; pos += 1;
        let _hit_sound_type = data[pos]; pos += 1;

        let name = read_str(&data, &mut pos, Some(item_id));
        let _texture = read_str(&data, &mut pos, None);
        let _texture_hash = read_u32(&data, &mut pos);
        let visual_effect = data[pos]; pos += 1;
        let _val1 = read_u32(&data, &mut pos);
        let _texture_x = data[pos]; pos += 1;
        let _texture_y = data[pos]; pos += 1;
        let _spread_type = data[pos]; pos += 1;
        let _is_stripey_wallpaper = data[pos]; pos += 1;
        let _collision_type = data[pos]; pos += 1;

        let break_hits_raw = data[pos]; pos += 1;
        let hits_to_break = if break_hits_raw % 6 == 0 { break_hits_raw / 6 } else { break_hits_raw };

        let _drop_chance = read_u32(&data, &mut pos);

        let clothing_type = data[pos]; pos += 1;

        let _rarity = read_u16(&data, &mut pos);
        let _max_amount = data[pos]; pos += 1;
        let _extra_file = read_str(&data, &mut pos, None);
        let _extra_file_hash = read_u32(&data, &mut pos);
        let _audio_volume = read_u32(&data, &mut pos);
        let _pet_name = read_str(&data, &mut pos, None);
        let _pet_prefix = read_str(&data, &mut pos, None);
        let _pet_suffix = read_str(&data, &mut pos, None);
        let _pet_ability = read_str(&data, &mut pos, None);
        let _seed_base = data[pos]; pos += 1;
        let _seed_overlay = data[pos]; pos += 1;
        let _tree_base = data[pos]; pos += 1;
        let _tree_leaves = data[pos]; pos += 1;


        let _seed_color_a = data[pos]; pos += 1;
        let _seed_color_r = data[pos]; pos += 1;
        let _seed_color_g = data[pos]; pos += 1;
        let _seed_color_b = data[pos]; pos += 1;
        let _seed_overlay_color_a = data[pos]; pos += 1;
        let _seed_overlay_color_r = data[pos]; pos += 1;
        let _seed_overlay_color_g = data[pos]; pos += 1;
        let _seed_overlay_color_b = data[pos]; pos += 1;

        let _ingredients = read_u32(&data, &mut pos);
        let _grow_time = read_u32(&data, &mut pos);
        let _val2 = read_u16(&data, &mut pos);
        let rayman = read_u16(&data, &mut pos);
        let _extra_options = read_str(&data, &mut pos, None);
        let _texture2 = read_str(&data, &mut pos, None);
        let _extra_options2 = read_str(&data, &mut pos, None);


        pos += 80;

        let mut punch_options = String::new();
        if version >= 11 {
            punch_options = read_str(&data, &mut pos, None);
        }
        if version >= 12 {
            pos += 13;
        }
        if version >= 13 {
            pos += 4;
        }
        if version >= 14 {
            pos += 4;
        }
        if version >= 15 {
             pos += 25;
             let _str_version_15 = read_str(&data, &mut pos, None);
        }
        if version >= 16 {
            let _str_version_16 = read_str(&data, &mut pos, None);
        }
        if version >= 17 {
            pos += 4;
        }
        if version >= 18 {
            pos += 4;
        }
        if version >= 19 {
            pos += 9;
        }
        if version >= 21 {
            pos += 2;
        }
        if version >= 22 {
            let _str_version_22 = read_str(&data, &mut pos, None);
        }
        if version >= 24 {
            pos += 5;
        }

        m.insert(item_id, ItemConfig {
            id: item_id,
            clothing_type,
            action_type,
            hits_to_break,
            is_breakable: hits_to_break > 0,
            is_background: action_type == 1,
            name,
            punch_effect: None,
            visual_effect,
            rayman,
            punch_options,
        });
    }

    m
}