//! SeaORM Entity. Generated by sea-orm-codegen 0.8.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "vsm_tbrowsersessions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(column_name = "sessionID")]
    pub session_id: String,
    pub time: i32,
    #[sea_orm(column_name = "lockedUntil")]
    pub locked_until: i32,
    pub error: Option<String>,
    #[sea_orm(column_name = "userID")]
    pub user_id: Option<i32>,
    #[sea_orm(column_name = "tSessionID")]
    pub t_session_id: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("No RelationDef")
    }
}

impl ActiveModelBehavior for ActiveModel {}
