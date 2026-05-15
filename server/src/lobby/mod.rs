mod room;

use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

pub use room::Room;

use crate::games;

pub struct Lobby {
    rooms: DashMap<Uuid, Arc<Room>>,
}

impl Lobby {
    pub fn new() -> Self {
        Self { rooms: DashMap::new() }
    }

    /// Create a new room. Returns `None` if `game_name` is not a known game.
    /// `victory_goal` is the cumulative session score a player must reach to win.
    pub fn create_room(
        &self,
        game_name: String,
        player_count: usize,
        victory_goal: i32,
    ) -> Option<Arc<Room>> {
        let game = games::get_game(&game_name)?;
        let room = Arc::new(Room::new(Uuid::new_v4(), game_name, player_count, game, victory_goal));
        self.rooms.insert(room.id, Arc::clone(&room));
        Some(room)
    }

    pub fn get_room(&self, id: Uuid) -> Option<Arc<Room>> {
        self.rooms.get(&id).map(|r| Arc::clone(&r))
    }

    pub fn remove_room(&self, id: Uuid) {
        self.rooms.remove(&id);
    }
}

impl Default for Lobby {
    fn default() -> Self {
        Self::new()
    }
}
