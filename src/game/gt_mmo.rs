use std::collections::HashMap;

pub const MAX_LEVEL: u32 = 200;

pub fn get_xp_required(level: u32) -> u32 {

    150 * level.pow(2) + 500 * level
}

pub fn get_milestone_title(level: u32) -> Option<&'static str> {
    match level {
        1 => Some("Beginner Farmer"),
        10 => Some("Farmer"),
        25 => Some("Skilled Farmer"),
        50 => Some("Expert Farmer"),
        75 => Some("Veteran Farmer"),
        100 => Some("Master Farmer"),
        150 => Some("Grand Farmer"),
        200 => Some("Legendary Farmer"),
        _ => None,
    }
}

pub fn get_block_xp(item_id: u32) -> u32 {
    match item_id {
        2 => 1,
        880 => 3,
        _ => 0,
    }
}

pub fn check_farmer_xp(player: &mut crate::database::player::Player, block_id: u32) -> (u32, bool) {
    let xp_gain = get_block_xp(block_id);
    if xp_gain == 0 {
        return (0, false);
    }


    if player.farmer_lvl >= MAX_LEVEL as i32 {
        return (0, false);
    }

    player.farmer_xp += xp_gain as i32;
    let required = get_xp_required(player.farmer_lvl as u32) as i32;


    if player.farmer_xp >= required {
        player.farmer_xp = 0;






        player.farmer_lvl += 1;
        return (xp_gain, true);
    }

    (xp_gain, false)
}