use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};

fn main() {
    let mut data = Vec::new();
    let mut f = File::open("items.dat").expect("Failed to open items.dat");
    f.read_to_end(&mut data).expect("Failed to read items.dat");

    let mut pos = 0;
    let mut output = File::create("particle_debug.txt").unwrap();

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
        String::from_utf8_lossy(&bytes).to_string()
    };

    let version = read_u16(&data, &mut pos);
    let item_count = read_u32(&data, &mut pos);

    writeln!(output, "Items.dat Version: {}, Count: {}", version, item_count).unwrap();

    for _ in 0..item_count {
        let item_id = read_u32(&data, &mut pos) as i32;

        pos += 4;

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

        let _break_hits_raw = data[pos]; pos += 1;
        let _drop_chance = read_u32(&data, &mut pos);
        let _clothing_type = data[pos]; pos += 1;
        let _rarity = read_u16(&data, &mut pos);
        let _max_amount = data[pos]; pos += 1;
        let _extra_file = read_str(&data, &mut pos, None);
        let _extra_file_hash = read_u32(&data, &mut pos);
        let _audio_volume = read_u32(&data, &mut pos);
        let _pet_name = read_str(&data, &mut pos, None);
        let _pet_prefix = read_str(&data, &mut pos, None);
        let _pet_suffix = read_str(&data, &mut pos, None);
        let _pet_ability = read_str(&data, &mut pos, None);
        pos += 4;
        pos += 8;
        let _ingredients = read_u32(&data, &mut pos);
        let _grow_time = read_u32(&data, &mut pos);
        let _val2 = read_u16(&data, &mut pos);
        let rayman = read_u16(&data, &mut pos);
        let _extra_options = read_str(&data, &mut pos, None);
        let _texture2 = read_str(&data, &mut pos, None);
        let _extra_options2 = read_str(&data, &mut pos, None);

        if visual_effect != 0 || rayman != 0 || item_id == 3064 || item_id == 5480 || item_id == 1550 || item_id == 1016 {
            writeln!(output, "Item ID: {}, Name: {}, Visual Effect: {}, Rayman: {}", item_id, name, visual_effect, rayman).unwrap();
        }

        let data_80 = &data[pos..pos+80];
        pos += 80;

        if version >= 11 {
            let punch_options = read_str(&data, &mut pos, None);
            if !punch_options.is_empty() {
                writeln!(output, "Item ID: {}, Name: {}, Punch Options: {}", item_id, name, punch_options).unwrap();
            }
        }
        if version >= 12 { pos += 13; }
        if version >= 13 { pos += 4; }
        if version >= 14 { pos += 4; }
        if version >= 15 {
             pos += 25;
             let _str_v15 = read_str(&data, &mut pos, None);
        }
        if version >= 16 {
            let _str_v16 = read_str(&data, &mut pos, None);
        }
        if version >= 17 { pos += 4; }
        if version >= 18 { pos += 4; }
        if version >= 19 { pos += 9; }
        if version >= 21 { pos += 2; }
        if version >= 22 {
            let _str_v22 = read_str(&data, &mut pos, None);
        }
        if version >= 24 { pos += 5; }
    }
}
