use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};
use tracing::{info, error, warn};
use rusqlite::Connection;
use crate::database::player::Player;
use crate::database::world::World;

pub enum DbCommand {
    UpdatePlayer(Player),
    UpdateWorld(World),
}

pub fn start_db_thread(rx: Receiver<DbCommand>) {
    info!("Starting Database Writer Thread...");

    let mut conn = match Connection::open("db/game_data.db") {
        Ok(c) => c,
        Err(e) => {
            error!("CRITICAL: Failed to open game_data.db in writer thread: {}", e);
            return;
        }
    };


    if let Err(e) = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;") {
        error!("Failed to set WAL mode: {}", e);
    }








    let mut conn_players = match Connection::open("db/peers.db") {
        Ok(c) => c,
        Err(e) => { error!("Failed to open peers.db: {}", e); return; }
    };

    let mut conn_worlds = match Connection::open("db/worlds.db") {
        Ok(c) => c,
        Err(e) => { error!("Failed to open worlds.db: {}", e); return; }
    };


    let _ = conn_players.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;");
    let _ = conn_worlds.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;");

    let mut pending_players: Vec<Player> = Vec::new();
    let mut pending_worlds: Vec<World> = Vec::new();
    let mut last_flush = Instant::now();
    let flush_interval = Duration::from_millis(200);
    let batch_limit = 100;

    loop {


        let timeout = flush_interval.checked_sub(last_flush.elapsed()).unwrap_or(Duration::ZERO);

        match rx.recv_timeout(timeout) {
            Ok(cmd) => {
                match cmd {
                    DbCommand::UpdatePlayer(p) => pending_players.push(p),
                    DbCommand::UpdateWorld(w) => pending_worlds.push(w),
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {

            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                info!("DB Channel disconnected. Flushing remaining and exiting.");
                flush(&mut conn_players, &mut conn_worlds, &mut pending_players, &mut pending_worlds);
                break;
            }
        }


        if last_flush.elapsed() >= flush_interval || pending_players.len() >= batch_limit || pending_worlds.len() >= batch_limit {
            flush(&mut conn_players, &mut conn_worlds, &mut pending_players, &mut pending_worlds);
            last_flush = Instant::now();
        }
    }
}

fn flush(
    conn_players: &mut Connection,
    conn_worlds: &mut Connection,
    pending_players: &mut Vec<Player>,
    pending_worlds: &mut Vec<World>
) {
    if pending_players.is_empty() && pending_worlds.is_empty() {
        return;
    }


    if !pending_players.is_empty() {
        let tx = match conn_players.transaction() {
            Ok(t) => t,
            Err(e) => { error!("Failed to start player transaction: {}", e); return; }
        };

        let mut count = 0;
        for p in pending_players.drain(..) {








            if let Err(e) = crate::database::player::save_player_internal(&tx, &p) {
                error!("Error saving player {}: {}", p.name, e);
            } else {
                count += 1;
            }
        }

        if let Err(e) = tx.commit() {
            error!("Failed to commit player batch: {}", e);
        } else {

        }
    }


    if !pending_worlds.is_empty() {
        let tx = match conn_worlds.transaction() {
            Ok(t) => t,
            Err(e) => { error!("Failed to start world transaction: {}", e); return; }
        };

        let mut count = 0;
        for w in pending_worlds.drain(..) {
            if let Err(e) = crate::database::world::save_world_internal(&tx, &w) {
                error!("Error saving world {}: {}", w.name, e);
            } else {
                count += 1;
            }
        }

        if let Err(e) = tx.commit() {
            error!("Failed to commit world batch: {}", e);
        } else {

        }
    }
}