use crate::database::world::{World, TileChangeResult};
use tracing::info;

pub fn handle_punch_effects(world: &mut World, start_x: i32, start_y: i32, direction: bool, equipped_items: &[i32], punched_block_id: i32) -> Vec<(i32, i32, TileChangeResult)> {
    let mut results = Vec::new();


    let hand_item = equipped_items.get(5).copied().unwrap_or(0);
    if hand_item == 0 { return results; }


    let config = crate::database::item_config::get_item_config(hand_item);


    if let Some(effect) = config.punch_effect {


        if !effect.allowed_targets.contains(&punched_block_id) {
            return results;
        }

        let step = if direction { -1 } else { 1 };

        for i in 1..=effect.range {
            let target_x = start_x + (i * step);
            let target_y = start_y;

            if target_x >= 0 && target_x < world.width as i32 {
                let idx = (target_y * world.width as i32 + target_x) as usize;
                if idx < world.tiles.len() {
                    let tile = &world.tiles[idx];
                    let fg = tile.fg as i32;
                    let bg = tile.bg as i32;




                    let mut hit = false;
                    if fg != 0 && effect.allowed_targets.contains(&fg) {
                        hit = true;
                    } else if bg != 0 && fg == 0 && effect.allowed_targets.contains(&bg) {
                        hit = true;
                    }

                    if hit {
                         let res = world.damage_tile(target_x, target_y);
                         if res != TileChangeResult::NoChange {
                            results.push((target_x, target_y, res));
                         }
                    }
                }
            }
        }
    }

    results
}