use http::{HeaderMap, HeaderValue, StatusCode, header::USER_AGENT};
use anyhow;

use crate::models::{IngameMetadataResponse, KeeperResponse, SearchResponse, SearchResult, StatsResponse};

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

pub async fn server_snapshot(server_guid: String) -> Result<KeeperResponse, anyhow::Error> {
    let res = reqwest::Client::new()
        .get(format!("https://keeper.battlelog.com/snapshot/{}", server_guid))
        .header(USER_AGENT, "BattleFox")
        .send()
        .await?;

    let status = res.status();

    let data_str = res
        .text()
        .await?;
    //println!("{}", data_str);

    if status != StatusCode::OK {
        return Err(anyhow::anyhow!(data_str));
    }

    let data: KeeperResponse = serde_json::from_str(&data_str)?;
    //let data = res.json::<KeeperResponse>().await?;
    //println!("KeeperResponse: {:#?}", data);

    Ok(data)
}

pub async fn ingame_metadata(persona_id: u64) -> Result<IngameMetadataResponse, anyhow::Error> {
    let res = reqwest::Client::new()
        .get(format!("https://battlelog.battlefield.com/api/bf4/pc/persona/1/{}/ingame_metadata", persona_id))
        .header(USER_AGENT, "BattleFox")
        .send()
        .await?;

    let status = res.status();

    let data_str = res
        .text()
        .await?;
    //println!("{}", data_str);

    if status != StatusCode::OK {
        return Err(anyhow::anyhow!(data_str));
    }

    let data: IngameMetadataResponse = serde_json::from_str(&data_str)?;
    //println!("IngameMetadataResponse: {:#?}", data);

    Ok(data)
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
