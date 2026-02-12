use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, error};
use bytes::{Buf, BufMut, BytesMut};
use anyhow::Result;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    pub fg: u16,
    pub bg: u16,
    pub state3: u8,
    pub state4: u8,
    pub hits: u8,
    pub label: String,
    pub last_tick: u64,
}

impl Tile {
    pub fn new(fg: u16, bg: u16) -> Self {
        Self {
            fg,
            bg,
            state3: 0,
            state4: 0,
            hits: 0,
            label: String::new(),
            last_tick: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub owner_name: String,
    pub owner: i32,
    pub tiles: Vec<Tile>,
    #[serde(default)]
    pub npcs: Vec<crate::game::npc::Npc>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TileChangeResult {
    NoChange,
    BrokeFG(u16, u8),
    BrokeBG(u16, u8),
    PlacedFG(u16),
    PlacedBG(u16),
    Damaged(u16, u8),
}

impl World {
    pub fn new(name: &str) -> Self {
        let mut world = Self {
            name: name.to_uppercase(),
            width: 100,
            height: 60,
            owner_name: String::new(),
            owner: 0,
            tiles: vec![Tile::new(0, 0); 6000],
            npcs: Vec::new(),
        };
        world.generate();
        world
    }

    pub fn generate(&mut self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let main_door_x = rng.gen_range(2..98);

        for y in 0..60 {
            for x in 0..100 {
                let i = (y * 100 + x) as usize;


                if y >= 37 {
                    self.tiles[i].bg = 14;

                    if y >= 38 && y < 50 {

                        if rng.gen_ratio(1, 38) {
                            self.tiles[i].fg = 10;
                        } else {
                            self.tiles[i].fg = 2;
                        }
                    } else if y >= 50 && y < 54 {

                        if rng.gen_ratio(3, 8) {
                            self.tiles[i].fg = 4;
                        } else {
                            self.tiles[i].fg = 2;
                        }
                    } else if y >= 54 {
                        self.tiles[i].fg = 8;
                    } else {
                        self.tiles[i].fg = 2;
                    }
                }


                if x == main_door_x && y == 36 {
                    self.tiles[i].fg = 6;
                    self.tiles[i].label = "EXIT".to_string();
                } else if x == main_door_x && y == 37 {
                    self.tiles[i].fg = 8;
                }
            }
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        use bytes::BufMut;
        let mut buf = Vec::new();


        buf.put_u16_le(0x14);
        buf.put_u32_le(0x40);


        buf.put_u16_le(self.name.len() as u16);
        buf.extend_from_slice(self.name.as_bytes());


        buf.put_u32_le(self.width);
        buf.put_u32_le(self.height);
        buf.put_u32_le(self.tiles.len() as u32);


        buf.extend_from_slice(&[0; 5]);


        for (_i, tile) in self.tiles.iter().enumerate() {

            let mut flags = tile.state3 as u16 | ((tile.state4 as u16) << 8);


            let config = crate::database::item_config::get_item_config(tile.fg as i32);
            let action_type = config.action_type;



            let has_extra_data = matches!(action_type, 2 | 3 | 10 | 13 | 19 | 26 | 33 | 34);

            if has_extra_data {
                flags |= 0x0001;
            }

            buf.put_u16_le(tile.fg);
            buf.put_u16_le(tile.bg);
            buf.put_u16_le(0);
            buf.put_u16_le(flags);

            if has_extra_data {
                match action_type {
                    2 | 26 => {
                        buf.put_u8(0x01);
                        buf.put_u16_le(tile.label.len() as u16);
                        buf.extend_from_slice(tile.label.as_bytes());
                        buf.put_u8(0);
                    }
                     13 => {
                        buf.put_u8(0x01);
                        buf.put_u16_le(tile.label.len() as u16);
                        buf.extend_from_slice(tile.label.as_bytes());
                        buf.put_u8(0);
                    }
                    10 => {
                        buf.put_u8(0x02);
                        buf.put_u16_le(tile.label.len() as u16);
                        buf.extend_from_slice(tile.label.as_bytes());
                        buf.put_i32_le(-1);
                    }
                    3 => {

                         buf.put_u8(0x03);
                         buf.put_u8(0);
                         buf.put_u32_le(0);
                         buf.put_u32_le(0);

                         buf.extend_from_slice(&[0; 8]);
                    }
                    19 => {
                        buf.put_u8(0x04);
                        buf.put_u32_le(0);
                        buf.put_u8(0);
                    }
                    33 | 34 => {

                         buf.put_u8(0x0);
                    }
                    _ => {



                    }
                }
            }
        }


        buf.extend_from_slice(&[0; 12]);
        buf.put_i32_le(0);
        buf.put_i32_le(0);


        buf.put_u16_le(41);
        buf.put_u16_le(0);
        buf.put_u32_le(0);
        buf.put_u32_le(0);

        buf
    }

    pub fn handle_tile_change(&mut self, packet: &mut crate::network::packet::GamePacket, player_name: &str) -> TileChangeResult {
        let x = packet.punch_x;
        let y = packet.punch_y;
        if x < 0 || x >= self.width as i32 || y < 0 || y >= self.height as i32 {
            return TileChangeResult::NoChange;
        }

        let i = (y * self.width as i32 + x) as usize;


        if self.owner_name.is_empty() {
             self.owner_name = player_name.to_string();
             info!("World {} now owned by {}", self.name, self.owner_name);
        }


        if !self.owner_name.is_empty() && self.owner_name != player_name {
            return TileChangeResult::NoChange;
        }

        let tile = &mut self.tiles[i];
        let item_held = packet.id;

        if item_held == 18 {

                return self.damage_tile(x, y);

        } else if item_held == 32 {

            return TileChangeResult::NoChange;
        } else {

            let config = crate::database::item_config::get_item_config(item_held);
            if config.is_background {
                if tile.bg == 0 {
                    tile.bg = item_held as u16;
                    tile.hits = 0;
                    return TileChangeResult::PlacedBG(tile.bg);
                }
            } else {
                if tile.fg == 0 {
                    tile.fg = item_held as u16;
                    tile.hits = 0;

                    if tile.fg == 12 || tile.fg == 6 {
                        tile.label = "EXIT".to_string();
                    }
                    return TileChangeResult::PlacedFG(tile.fg);
                }
            }
        }

        TileChangeResult::NoChange
    }

    pub fn serialize_to_binary(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        buf.put_i32_le(1);


        buf.put_u16_le(self.name.len() as u16);
        buf.extend_from_slice(self.name.as_bytes());


        buf.put_u32_le(self.width);
        buf.put_u32_le(self.height);


        buf.put_i32_le(self.owner);
        buf.put_u16_le(self.owner_name.len() as u16);
        buf.extend_from_slice(self.owner_name.as_bytes());


        buf.put_u32_le(self.tiles.len() as u32);
        for tile in &self.tiles {
            buf.put_u16_le(tile.fg);
            buf.put_u16_le(tile.bg);
            buf.put_u8(tile.hits);
            buf.put_u8(tile.state3);
            buf.put_u8(tile.state4);
            buf.put_u64_le(tile.last_tick);

            buf.put_u16_le(tile.label.len() as u16);
            if !tile.label.is_empty() {
                buf.extend_from_slice(tile.label.as_bytes());
            }
        }

        buf
    }

    pub fn deserialize_from_binary(data: &[u8]) -> Result<Self> {
        let mut buf = &data[..];
        let _version = buf.get_i32_le();

        let name_len = buf.get_u16_le() as usize;
        let name = String::from_utf8(buf.copy_to_bytes(name_len).to_vec())?;

        let width = buf.get_u32_le();
        let height = buf.get_u32_le();

        let owner = buf.get_i32_le();
        let owner_name_len = buf.get_u16_le() as usize;
        let owner_name = String::from_utf8(buf.copy_to_bytes(owner_name_len).to_vec())?;

        let tile_count = buf.get_u32_le() as usize;
        let mut tiles = Vec::with_capacity(tile_count);

        for _ in 0..tile_count {
            let fg = buf.get_u16_le();
            let bg = buf.get_u16_le();
            let hits = buf.get_u8();
            let state3 = buf.get_u8();
            let state4 = buf.get_u8();
            let last_tick = buf.get_u64_le();

            let label_len = buf.get_u16_le() as usize;
            let label = if label_len > 0 {
                String::from_utf8(buf.copy_to_bytes(label_len).to_vec())?
            } else {
                String::new()
            };

            tiles.push(Tile {
                fg, bg, state3, state4, hits, label, last_tick
            });
        }

        Ok(Self {
            name, width, height, owner_name, owner, tiles,
            npcs: Vec::new(),
        })
    }

    pub fn damage_tile(&mut self, x: i32, y: i32) -> TileChangeResult {
        if x < 0 || x >= self.width as i32 || y < 0 || y >= self.height as i32 {
            return TileChangeResult::NoChange;
        }
        let i = (y * self.width as i32 + x) as usize;
        let tile = &mut self.tiles[i];

        if tile.fg != 0 {

            if tile.fg == 6 || tile.fg == 8 { return TileChangeResult::NoChange; }

            let config = crate::database::item_config::get_item_config(tile.fg as i32);
            if !config.is_breakable { return TileChangeResult::NoChange; }

            tile.hits += 1;
            let current_hits = tile.hits;
            tracing::info!("HITS: FG {} at {},{} now {}/{}", tile.fg, x, y, current_hits, config.hits_to_break);

            if current_hits >= config.hits_to_break {
                let id_before = tile.fg;
                tile.fg = 0;
                tile.hits = 0;
                tile.label = String::new();

                return TileChangeResult::BrokeFG(id_before, current_hits);
            }

            return TileChangeResult::Damaged(tile.fg, current_hits);
        } else if tile.bg != 0 {

            let config = crate::database::item_config::get_item_config(tile.bg as i32);
            if !config.is_breakable { return TileChangeResult::NoChange; }

            tile.hits += 1;
            let current_hits = tile.hits;
            tracing::info!("HITS: BG {} at {},{} now {}/{}", tile.bg, x, y, current_hits, config.hits_to_break);

            if current_hits >= config.hits_to_break {
                let id_before = tile.bg;
                tile.bg = 0;
                tile.hits = 0;
                tile.label = String::new();

                return TileChangeResult::BrokeBG(id_before, current_hits);
            }

            return TileChangeResult::Damaged(tile.bg, current_hits);
        }
        TileChangeResult::NoChange
    }
}

pub fn init_db() -> Result<(), rusqlite::Error> {
    let conn = rusqlite::Connection::open("db/worlds.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS worlds (
            name TEXT PRIMARY KEY,
            owner_name TEXT,
            owner INTEGER,
            data BLOB
        )",
        [],
    )?;
    Ok(())
}

pub fn save_world(world: &World) -> Result<()> {
    let mut conn = rusqlite::Connection::open("db/worlds.db")?;
    let tx = conn.transaction()?;
    save_world_internal(&tx, world)?;
    tx.commit()?;
    Ok(())
}

pub fn save_world_internal(tx: &rusqlite::Transaction, world: &World) -> Result<()> {

    let bin_data = world.serialize_to_binary();


    let compressed_data = zstd::encode_all(&bin_data[..], 3)?;

    tx.execute(
        "INSERT OR REPLACE INTO worlds (name, owner_name, owner, data) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![world.name, world.owner_name, world.owner, compressed_data],
    )?;
    Ok(())
}

pub fn load_world(name: &str) -> Result<Option<World>> {
    let conn = rusqlite::Connection::open("db/worlds.db")?;
    let mut stmt = conn.prepare("SELECT data FROM worlds WHERE name = ?1")?;
    let mut rows = stmt.query(rusqlite::params![name.to_uppercase()])?;

    if let Some(row) = rows.next()? {
        let compressed_data: Vec<u8> = row.get(0)?;


        let bin_data = zstd::decode_all(&compressed_data[..])?;


        let world = World::deserialize_from_binary(&bin_data)?;
        Ok(Some(world))
    } else {
        Ok(None)
    }
}