use crate::{BfoxContext, DateTime};

#[derive(Debug)]
pub struct BfoxMutedPlayer {
    pub eaid: String, // TODO: use the EAID type
    pub type_: i32, // TODO: use an enum.
    pub end_date: Option<DateTime>,
    pub kicks: Option<i32>, // TODO: use u32
    pub reason: Option<String>,
}

impl BfoxContext {
    pub async fn get_muted_players(&self) -> Result<Vec<BfoxMutedPlayer>, sqlx::Error> {
        todo!()
        // use schema::bfox_muted_players::dsl::{type_, end_date, bfox_muted_players};
        // bfox_muted_players
        //     .filter(type_.eq(MuteType::Days as i32).and(end_date.gt(Some(Utc::now().naive_utc().date()))))
        //     .or_filter(type_.ne(MuteType::Days as i32))
        //     .filter(type_.ne(MuteType::Disabled as i32))
        //     .load(conn)
    }

    pub async fn get_muted_player(&self, _id: impl AsRef<str>) -> Result<BfoxMutedPlayer, sqlx::Error> {
        todo!()
    //     use schema::bfox_muted_players::dsl::{eaid, bfox_muted_players};
    //     bfox_muted_players
    //         .filter(eaid.eq(id))
    //         .first(conn)
    }

    pub async fn replace_into_muted_player(&self, _player: &BfoxMutedPlayer) -> Result<(), sqlx::Error> {
        todo!()
    //     use schema::bfox_muted_players::dsl::*;
    //     // let primary_key = player.eaid.clone();
    //     // let exists = bfox_muted_players
    //     //     .filter(eaid.eq(primary_key))
    //     //     .count()
    //     //     .first::<i64>(conn)
    //     //     .unwrap();
    //     // println!("Count: {}", exists);
    //     replace_into(bfox_muted_players)
    //         .values(player)
    //         .execute(conn)?;
    //     Ok(())
    }

    pub async fn delete_muted_players(&self, _ids: &[impl AsRef<str>]) -> Result<(), sqlx::Error> {
        todo!()
    //     use schema::bfox_muted_players::dsl::*;
    //     delete(bfox_muted_players)
    //         .filter(eaid.eq(id))
    //         .execute(conn)?;
    //     Ok(())
    }
}