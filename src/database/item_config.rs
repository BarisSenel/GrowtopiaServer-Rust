
use std::collections::HashMap;
use once_cell::sync::Lazy;
use tracing::info;

#[derive(Debug, Clone)]
pub struct ItemConfig {
    pub id: i32,
    pub clothing_type: u8,
    pub action_type: u8,
    pub hits_to_break: u8,
    pub is_breakable: bool,
    pub is_background: bool,
    pub name: String,
    pub punch_effect: Option<PunchEffect>,
    pub visual_effect: u8,
    pub rayman: u16,
    pub punch_options: String,
}

impl ItemConfig {
    pub fn get_effects(&self) -> ItemEffects {
        let mut effects = ItemEffects::default();
        if self.punch_options.is_empty() {
            return effects;
        }

        for part in self.punch_options.split(';') {
            if let Some(particle_str) = part.strip_prefix("op_particle2:") {
                if let Ok(id) = particle_str.parse::<i32>() {
                    effects.particle_id = Some(id);
                }
            } else if let Some(audio_path) = part.strip_prefix("op_audio:") {
                effects.audio_path = Some(audio_path.to_string());
            }
        }
        effects
    }
}

#[derive(Debug, Clone)]
pub struct PunchEffect {
    pub range: i32,
    pub allowed_targets: Vec<i32>,
}

#[derive(Debug, Clone, Default)]
pub struct ItemEffects {
    pub particle_id: Option<i32>,
    pub audio_path: Option<String>,
}

pub static ITEMS: Lazy<HashMap<i32, ItemConfig>> = Lazy::new(|| {
    load_item_definitions()
});

fn load_item_definitions() -> HashMap<i32, ItemConfig> {

    let mut m = crate::database::items_decoder::load_item_definitions();




    apply_custom_overrides(&mut m);

    info!("Loaded {} items.", m.len());
    m
}

fn apply_custom_overrides(m: &mut HashMap<i32, ItemConfig>) {


    let mut update = |id: i32, hits: u8, breakable: bool, bg: bool| {
        m.entry(id).and_modify(|i| {
            i.hits_to_break = hits;
            i.is_breakable = breakable;
            i.is_background = bg;
        }).or_insert_with(|| ItemConfig {
            id,
            clothing_type: 0,
            action_type: if bg { 1 } else { 0 },
            hits_to_break: hits,
            is_breakable: breakable,
            is_background: bg,
            name: "Custom Item".to_string(),
            punch_effect: None,
            visual_effect: 0,
            rayman: 0,
            punch_options: String::new(),
        });
    };





    update(  2,   2,    true,      false);
    update(  4,   3,    true,      false);
    update(  6, 255,    false,     false);
    update(  8, 255,    false,     false);
    update( 10,   2,    true,      false);
    update( 12,   3,    true,      false);
    update( 880,   1,    true,      false);
    update( 14,   2,    true,      true );




    let mut add_effect = |id: i32, range: i32, targets: Vec<i32>| {
        m.entry(id).and_modify(|i| {
            i.punch_effect = Some(PunchEffect { range, allowed_targets: targets });
        });
    };




    add_effect(1068, 3, vec![880]);
}

pub fn get_item_config(id: i32) -> ItemConfig {
    ITEMS.get(&id).cloned().unwrap_or(ItemConfig {
        id,
        clothing_type: 0,
        action_type: 0,
        hits_to_break: 255,
        is_breakable: false,
        is_background: false,
        name: "Unknown".to_string(),
        punch_effect: None,
        visual_effect: 0,
        rayman: 0,
        punch_options: String::new(),
    })
}

pub fn get_clothing_type(id: i32) -> Option<usize> {
    let conf = get_item_config(id);


    if conf.action_type != 20 { return None; }



    if conf.clothing_type <= 9 {
        Some(conf.clothing_type as usize)
    } else {
        None
    }
}