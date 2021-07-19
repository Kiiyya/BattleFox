use crate::models::{Player, Snapshot};

impl Snapshot {
    pub fn get_player_by_personaid(&self, persona_id: &u64) -> Option<Player> {
        let mut snapshot = self.clone();
        for teaminfo in snapshot.team_info.values_mut() {
            if teaminfo.players.contains_key(persona_id) {
                return teaminfo.players.remove(persona_id);
            }
        }

        None
    }

    pub fn get_player_by_name(&self, name: &str) -> Option<Player> {
        let mut snapshot = self.clone();
        for teaminfo in snapshot.team_info.values_mut() {
            teaminfo.players.retain(|_pid, player| player.name.eq(name));

            if !teaminfo.players.is_empty() {
                match teaminfo.players.keys().next() {
                    Some(&x) => return teaminfo.players.remove(&x),
                    _ => continue,
                }
            }
        }

        None
    }
}