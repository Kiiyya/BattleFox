use http::{header::USER_AGENT, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_aux::prelude::*;
use std::collections::HashMap;
use anyhow;

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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub persona_id: u64,
    pub user: User,
}

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

pub async fn search_user(soldier_name: String) -> Result<SearchResult, anyhow::Error> {
    let params = [("query", soldier_name.clone())];
    let client = reqwest::Client::new();
    let res = client
        .post("https://battlelog.battlefield.com/bf4/search/query/")
        .form(&params)
        .header(USER_AGENT, "BattleFox")
        .send()
        .await?;

    // let t = res
    //     .text()
    //     .await?;
    // println!("{}", t);

    let mut js = res.json::<SearchResponse>().await?;
    //println!("SearchResponse: {:#?}", js);

    for i in 0..js.data.len() {
        let result = &js.data[i];
        //println!("User: {:#?}", result);

        // Requires correct persona name. Apparently default parameters or overrides are not supported so not adding support for partial names now.
        if result.persona_name.ne(&soldier_name) {
            //println!("Not a correct persona");
            continue;
        }

        if result.namespace != "cem_ea_id" {
            //println!("Not a PC namespace");
            continue;
        }

        for val in result.games.values() {
            if val.parse::<i32>().unwrap() & 2048 == 0 {
                continue;
            }
            //println!("Has BF4");

            return Ok(js.data.remove(i))
        }
    }

    Err(anyhow::anyhow!("User not found"))
}

pub async fn get_user(persona_id: String) -> Result<StatsResponse, anyhow::Error> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_str("BattleFox").unwrap());
    headers.insert("X-AjaxNavigation", HeaderValue::from_str("1").unwrap());
    headers.insert(
        "X-Requested-With",
        HeaderValue::from_str("XMLHttpRequest").unwrap(),
    );

    let res = reqwest::Client::new()
        .get(format!(
            "https://battlelog.battlefield.com/bf4/soldier/SOLDIER/stats/{}/pc/",
            persona_id
        ))
        .headers(headers)
        .send()
        .await?;

    // let t = res
    //     .text()
    //     .await?;

    // // println!("{}", t);

    // let data: StatsResponse = serde_json::from_str(&t).unwrap();
    // println!("{:#?}", data);

    let js = res.json::<StatsResponse>().await?;

    //println!("JS: {:#?}", js);

    Ok(js)
}
