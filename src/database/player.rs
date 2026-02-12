use rusqlite::{params, Connection, Result};
use tracing::info;

#[derive(Debug, Clone)]
pub struct InventorySlot {
    pub item_id: i32,
    pub count: i32,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub name: String,
    pub role: i32,
    pub gems: i32,
    pub level: i32,
    pub xp: i32,
    pub slots: Vec<InventorySlot>,
    pub equipped: Vec<i32>,
    pub discord_id: Option<String>,
    pub discord_username: Option<String>,
    pub email: Option<String>,
    pub ltoken: Option<String>,
    pub skin_color: u32,

    pub farmer_lvl: i32,
    pub farmer_xp: i32,
    pub miner_lvl: i32,
    pub miner_xp: i32,
    pub adventurer_lvl: i32,
    pub adventurer_xp: i32,
    pub punch_id: u8,
}

impl Player {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            role: 1,
            gems: 0,
            level: 1,
            xp: 0,
            slots: vec![
                InventorySlot { item_id: 18, count: 1 },
                InventorySlot { item_id: 32, count: 1 },
            ],
            equipped: vec![0; 10],
            discord_id: None,
            discord_username: None,
            email: None,
            ltoken: None,
            skin_color: 3370516479,
            farmer_lvl: 1, farmer_xp: 0,
            miner_lvl: 1, miner_xp: 0,
            adventurer_lvl: 1, adventurer_xp: 0,
            punch_id: 0,
        }
    }
}

pub fn init_db() -> Result<()> {
    let conn = Connection::open("db/peers.db")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS peers (
            _n TEXT PRIMARY KEY,
            role INTEGER,
            gems INTEGER,
            lvl INTEGER,
            xp INTEGER,
            discord_id TEXT,
            discord_username TEXT,
            email TEXT,
            ltoken TEXT,
            skin_color INTEGER,
            farmer_lvl INTEGER,
            farmer_xp INTEGER,
            miner_lvl INTEGER,
            miner_xp INTEGER,
            adventurer_lvl INTEGER,
            adventurer_xp INTEGER
        )",
        [],
    )?;


    let _ = conn.execute("ALTER TABLE peers ADD COLUMN discord_id TEXT", []);
    let _ = conn.execute("ALTER TABLE peers ADD COLUMN discord_username TEXT", []);
    let _ = conn.execute("ALTER TABLE peers ADD COLUMN email TEXT", []);
    let _ = conn.execute("ALTER TABLE peers ADD COLUMN ltoken TEXT", []);
    let _ = conn.execute("ALTER TABLE peers ADD COLUMN skin_color INTEGER DEFAULT 3370516479", []);

    let _ = conn.execute("ALTER TABLE peers ADD COLUMN farmer_lvl INTEGER DEFAULT 1", []);
    let _ = conn.execute("ALTER TABLE peers ADD COLUMN farmer_xp INTEGER DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE peers ADD COLUMN miner_lvl INTEGER DEFAULT 1", []);
    let _ = conn.execute("ALTER TABLE peers ADD COLUMN miner_xp INTEGER DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE peers ADD COLUMN adventurer_lvl INTEGER DEFAULT 1", []);
    let _ = conn.execute("ALTER TABLE peers ADD COLUMN adventurer_xp INTEGER DEFAULT 0", []);

    conn.execute(
        "CREATE TABLE IF NOT EXISTS slots (
            _n TEXT,
            i INTEGER,
            c INTEGER,
            FOREIGN KEY(_n) REFERENCES peers(_n)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS equip (
            _n TEXT,
            i INTEGER,
            s INTEGER,
            FOREIGN KEY(_n) REFERENCES peers(_n)
        )",
        [],
    )?;


    let _ = conn.execute("ALTER TABLE equip ADD COLUMN s INTEGER", []);

    info!("Database initialized successfully");
    Ok(())
}

pub fn load_player(name: &str) -> Result<Option<Player>> {
    let conn = Connection::open("db/peers.db")?;

    let mut stmt = conn.prepare("SELECT role, gems, lvl, xp, discord_id, discord_username, email, ltoken, skin_color, farmer_lvl, farmer_xp, miner_lvl, miner_xp, adventurer_lvl, adventurer_xp FROM peers WHERE _n = ?")?;
    let mut rows = stmt.query(params![name])?;

    if let Some(row) = rows.next()? {
        let mut player = Player::new(name);
        player.role = row.get(0)?;
        player.gems = row.get(1)?;
        player.level = row.get(2)?;
        player.xp = row.get(3)?;
        player.discord_id = row.get(4)?;
        player.discord_username = row.get(5)?;
        player.email = row.get(6)?;
        player.ltoken = row.get(7)?;
        player.skin_color = row.get(8).unwrap_or(3370516479);
        player.farmer_lvl = row.get(9).unwrap_or(1);
        player.farmer_xp = row.get(10).unwrap_or(0);
        player.miner_lvl = row.get(11).unwrap_or(1);
        player.miner_xp = row.get(12).unwrap_or(0);
        player.adventurer_lvl = row.get(13).unwrap_or(1);
        player.adventurer_xp = row.get(14).unwrap_or(0);


        player.slots.clear();


        let mut slot_stmt = conn.prepare("SELECT i, c FROM slots WHERE _n = ?")?;
        let slot_rows = slot_stmt.query_map(params![name], |r| {
            Ok(InventorySlot {
                item_id: r.get(0)?,
                count: r.get(1)?,
            })
        })?;

        for slot in slot_rows {
            player.slots.push(slot?);
        }


        let mut equip_stmt = conn.prepare("SELECT i, s FROM equip WHERE _n = ?")?;
        let equip_rows = equip_stmt.query_map(params![name], |r| {
            Ok((r.get::<_, i32>(0)?, r.get::<_, i32>(1)?))
        })?;


        player.equipped = vec![0; 10];

        for row in equip_rows {
            if let Ok((item_id, slot_idx)) = row {
                info!("DB Loaded item {} for slot {}", item_id, slot_idx);
                if slot_idx >= 0 && (slot_idx as usize) < player.equipped.len() {
                    player.equipped[slot_idx as usize] = item_id;
                }
            }
        }

        if player.equipped.len() < 10 { player.equipped.resize(10, 0); }

        Ok(Some(player))
    } else {
        Ok(None)
    }
}

pub fn save_player(player: &Player) -> Result<()> {
    let mut conn = Connection::open("db/peers.db")?;
    let tx = conn.transaction()?;
    save_player_internal(&tx, player)?;
    tx.commit()?;
    info!("Player {} saved to database (Sync)", player.name);
    Ok(())
}

pub fn save_player_internal(tx: &rusqlite::Transaction, player: &Player) -> Result<()> {
    tx.execute(
        "INSERT OR REPLACE INTO peers (_n, role, gems, lvl, xp, discord_id, discord_username, email, ltoken, skin_color, farmer_lvl, farmer_xp, miner_lvl, miner_xp, adventurer_lvl, adventurer_xp) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            player.name,
            player.role,
            player.gems,
            player.level,
            player.xp,
            player.discord_id,
            player.discord_username,
            player.email,
            player.ltoken,
            player.skin_color,
            player.farmer_lvl,
            player.farmer_xp,
            player.miner_lvl,
            player.miner_xp,
            player.adventurer_lvl,
            player.adventurer_xp
        ],
    )?;


    tx.execute("DELETE FROM slots WHERE _n = ?", params![player.name])?;
    for slot in &player.slots {
        if slot.count > 0 {
            tx.execute(
                "INSERT INTO slots (_n, i, c) VALUES (?, ?, ?)",
                params![player.name, slot.item_id, slot.count],
            )?;
        }
    }


    tx.execute("DELETE FROM equip WHERE _n = ?", params![player.name])?;
    for (slot_idx, &equip) in player.equipped.iter().enumerate() {
        if equip != 0 {
             tx.execute(
                "INSERT INTO equip (_n, i, s) VALUES (?, ?, ?)",
                params![player.name, equip, slot_idx as i32],
            )?;
        }
    }
    Ok(())
}

pub fn player_exists(name: &str) -> Result<bool> {
    let conn = Connection::open("db/peers.db")?;
    let mut stmt = conn.prepare("SELECT 1 FROM peers WHERE _n = ? LIMIT 1")?;
    let exists = stmt.exists(params![name])?;
    Ok(exists)
}

pub fn get_player_by_discord_id(discord_id: &str) -> Result<Option<Player>> {
    let conn = Connection::open("db/peers.db")?;
    let mut stmt = conn.prepare("SELECT _n FROM peers WHERE discord_id = ? LIMIT 1")?;
    let name: Option<String> = stmt.query_row(params![discord_id], |row| row.get(0)).optional()?;

    match name {
        Some(n) => load_player(&n),
        None => Ok(None),
    }
}

pub fn get_player_by_ltoken(ltoken: &str) -> Result<Option<Player>> {
    let conn = Connection::open("db/peers.db")?;
    let mut stmt = conn.prepare("SELECT _n FROM peers WHERE ltoken = ? LIMIT 1")?;
    let name: Option<String> = stmt.query_row(params![ltoken], |row| row.get(0)).optional()?;

    match name {
        Some(n) => load_player(&n),
        None => Ok(None),
    }
}

pub trait OptionalRow {
    fn optional(self) -> Result<Option<String>>;
}

impl OptionalRow for Result<String, rusqlite::Error> {
    fn optional(self) -> Result<Option<String>> {
        match self {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}