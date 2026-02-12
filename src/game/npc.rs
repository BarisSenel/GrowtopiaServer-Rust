use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Npc {
    pub net_id: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub health: i32,
    pub max_health: i32,
    pub target_x: f32,
    pub state: u32,
    pub last_jump: std::time::SystemTime,
}

impl Npc {
    pub fn new(net_id: u32, name: String, x: f32, y: f32, health: i32) -> Self {
        Self {
            net_id,
            name,
            x,
            y,
            health,
            max_health: health,
            target_x: x,
            state: 0,
            last_jump: std::time::SystemTime::now(),
        }
    }
}