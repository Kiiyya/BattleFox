use serde::{Deserialize, Serialize};
use serde_aux::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub username: Option<String>,
    pub gravatar_md5: Option<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub user_id: u64,
    pub created_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub persona_id: u64,
    pub user: User,
}

// #[derive(Debug, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct Game {
//     #[serde(deserialize_with = "deserialize_number_from_string")]
//     pub persona_id: u64,
//     pub user: User,
// }

/// # Example
/// ```ron
/// SearchResult {
///     picture: "",
///     user_id: 2955058489260500539,
///     user: User {
///         username: Some(
///             "PocketWolfy",
///         ),
///         gravatar_md5: Some(
///             "b97c726c98f9f615bd62088c9e4c5cb4",
///         ),
///         user_id: 2955058489260500539,
///         created_at: 1393081344,
///     },
///     persona_id: 994520424,
///     persona_name: "PocketWolfy",
///     namespace: "cem_ea_id",
///     games: {
///         1: "2050",
///     },
/// }
/// ```
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub picture: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub user_id: u64,
    pub user: User,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub persona_id: u64,
    pub persona_name: String,
    pub namespace: String,
    pub games: HashMap<i32, String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsResponse {
    pub template: String,
    pub context: Context,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub r#type: String,
    pub message: String,
    pub data: Vec<SearchResult>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeeperResponse {
    pub last_updated: u32,
    pub snapshot: Snapshot,
}

/// # Example
/// ```ron
/// KeeperResponse {
///     last_updated: 1626814,
///     snapshot: Snapshot {
///         status: "SUCCESS",
///         game_id: 18014398528206305,
///         game_mode: "RushLarge",
///         map_variant: 0,
///         current_map: "XP0/Levels/XP1_002_Oman/XP0_Oman",
///         max_players: 64,
///         waiting_players: 2,
///         round_time: 348,
///         default_round_time_multiplier: 100,
///         rush: Some(
///             Rush {
///                 defenders: Defenders {
///                     team: 2,
///                     bases: 2,
///                     bases_max: 3,
///                     attacker: 0,
///                 },
///                 attackers: Attackers {
///                     team: 1,
///                     tickets: 163,
///                     tickets_max: 300,
///                     attacker: 1,
///                 },
///             },
///         ),
///         conquest: None,
///         deathmatch: None,
///         carrier_assault: None,
///         team_info: {
///             2: TeamInfo {
///                 faction: 1,
///                 players: {
///                     994520424: Player {
///                         name: "PocketWolfy",
///                         tag: "Kiss",
///                         rank: 140,
///                         score: 213,
///                         kills: 1,
///                         deaths: 1,
///                         squad: 6,
///                         role: 1,
///                     },
///                     // ...
///                 },
///             },
///             0: TeamInfo {
///                 faction: 0,
///                 players: { /* ... */ },
///             },
///             1: TeamInfo {
///                 faction: 0,
///                 players: { /* ... */ },
///             },
///         },
///     },
/// }
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub status: String,
    pub game_id: u64,
    pub game_mode: String,
    pub map_variant: u8,
    pub current_map: String,
    pub max_players: u8,
    pub waiting_players: u8,
    pub round_time: u32,
    pub default_round_time_multiplier: u32,
    pub rush: Option<Rush>,
    pub conquest: Option<HashMap<u8, Conquest>>,
    pub deathmatch: Option<HashMap<u8, Deathmatch>>,
    pub carrier_assault: Option<HashMap<u8, CarrierAssault>>,
    // TODO: Add rest of the game modes
    pub team_info: HashMap<u8, TeamInfo>,
}

impl Snapshot {
    pub fn get_player_by_personaid(&self, persona_id: u64) -> Option<&Player> {
        self.team_info.values()
            .find_map(|teaminfo| teaminfo.players.get(&persona_id))
    }

    pub fn get_player_by_name(&self, name: &str) -> Option<&Player> {
        self.team_info.values()
            .flat_map(|ti| ti.players.values())
            .find(|p| p.name == name)
    }
}

//#region Game modes
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Rush {
    pub defenders: Defenders,
    pub attackers: Attackers
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Defenders {
    pub team: u8,
    pub bases: u8,
    pub bases_max: u8,
    pub attacker: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Attackers {
    pub team: u8,
    pub tickets: u16,
    pub tickets_max: u16,
    pub attacker: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Conquest {
    pub tickets: u32,
    pub tickets_max: u32
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Deathmatch {
    pub kills: u32,
    pub kills_max: u32
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CarrierAssault {
    pub destroyed_crates: u8,
    pub carrier_health: u8
}
//#endregion

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TeamInfo {
    pub faction: u8,
    pub players: HashMap<u64, Player>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub name: String,
    pub tag: String,
    pub rank: i16,
    pub score: u32,
    pub kills: u32,
    pub deaths: u32,
    pub squad: i8,
    pub role: u8
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngameMetadataResponse {
    pub club_rank: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub persona_id: u64,
    pub emblem_url: String,
    pub club_name: String,
    pub country_code: String,
}

impl IngameMetadataResponse {
    pub fn get_emblem_url(&self) -> Option<String> {
        if self.emblem_url.is_empty() {
            return None;
        }

        Some(self.emblem_url.replace(".dds", ".png"))
    }
}
