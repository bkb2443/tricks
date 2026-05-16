pub mod matchmaker;
mod room;

use dashmap::DashMap;
use rand::Rng;
use std::sync::Arc;

pub use matchmaker::Matchmaker;
pub use room::Room;

use crate::games;

const WORDS: &[&str] = &[
    "WOLF", "BEAR", "HAWK", "DUCK", "DEER", "CROW", "FROG", "LYNX",
    "MOOSE", "PIKE", "LARK", "WREN", "NEWT", "MINK", "VOLE", "IBIS",
    "KITE", "TEAL", "DOVE", "SWAN",
];

pub fn generate_room_code() -> String {
    let mut rng = rand::thread_rng();
    let word = WORDS[rng.gen_range(0..WORDS.len())];
    let num = rng.gen_range(10u32..=99);
    format!("{word}-{num:02}")
}

pub struct Lobby {
    rooms: DashMap<String, Arc<Room>>,
    pub matchmaker: std::sync::OnceLock<Arc<Matchmaker>>,
}

impl Lobby {
    pub fn new() -> Self {
        Self {
            rooms: DashMap::new(),
            matchmaker: std::sync::OnceLock::new(),
        }
    }

    /// Create a new room. Returns `(room_code, Arc<Room>)` or `None` if game_name unknown.
    pub fn create_room(
        &self,
        game_name: String,
        player_count: usize,
        victory_goal: i32,
    ) -> Option<(String, Arc<Room>)> {
        let game = games::get_game(&game_name)?;
        // Generate a unique code (retry on collision, up to 10 times)
        let code = (0..10)
            .map(|_| generate_room_code())
            .find(|c| !self.rooms.contains_key(c.as_str()))
            .unwrap_or_else(generate_room_code);
        let room = Arc::new(Room::new(
            uuid::Uuid::new_v4(),
            game_name,
            player_count,
            game,
            victory_goal,
            code.clone(),
            "private".into(),
        ));
        self.rooms.insert(code.clone(), Arc::clone(&room));
        Some((code, room))
    }

    pub fn get_room(&self, code: &str) -> Option<Arc<Room>> {
        self.rooms.get(code).map(|r| Arc::clone(&r))
    }

    #[allow(dead_code)]
    pub fn remove_room(&self, code: &str) {
        self.rooms.remove(code);
    }
}

impl Default for Lobby {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_code_format() {
        let code = generate_room_code();
        let parts: Vec<&str> = code.split('-').collect();
        assert_eq!(parts.len(), 2, "code should be WORD-NN format, got: {code}");
        let num: u32 = parts[1].parse().expect("second part should be a number");
        assert!((10..=99).contains(&num), "number should be 10-99, got: {num}");
        assert!(parts[0].len() >= 3, "word should be at least 3 chars");
    }

    #[test]
    fn get_room_by_code_round_trips() {
        let lobby = Lobby::new();
        let (code, _room) = lobby.create_room("sheepshead".into(), 5, 24).unwrap();
        assert!(lobby.get_room(&code).is_some(), "should find room by code");
        assert!(lobby.get_room("NOTEXIST-00").is_none());
    }

    #[test]
    fn create_room_returns_unique_codes() {
        let lobby = Lobby::new();
        let (code1, _) = lobby.create_room("sheepshead".into(), 5, 24).unwrap();
        let (code2, _) = lobby.create_room("sheepshead".into(), 5, 24).unwrap();
        assert_ne!(code1, code2, "each room should get a unique code");
    }
}
