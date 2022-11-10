pub mod models;

use http::{header::USER_AGENT, HeaderMap, HeaderValue, StatusCode};
pub use models::*;

pub async fn get_users(soldier_names: Vec<String>) -> Result<Vec<UserResult>, anyhow::Error> {
    let mut params = vec![("kind", "light")];

    for soldier_name in soldier_names.iter() {
        params.push(("personaNames", soldier_name));
    }

    let client = reqwest::Client::new();
    let res = client
        .get("https://battlelog.battlefield.com/bf4/battledash/getUsersByPersonaNames")
        .query(&params)
        .header(USER_AGENT, "BattleFox")
        .send()
        .await?;

    let status = res.status();
    let data_str = res.text().await?;
    // println!("{}", data_str);

    if status != StatusCode::OK {
        return Err(anyhow::anyhow!(data_str));
    }

    let response_data = match serde_json::from_str::<UsersResponse>(&data_str) {
        Ok(data) => data,
        Err(err) => return Err(anyhow::anyhow!("{} - {}", err, data_str)),
    };

    // let response_data = res.json::<UsersResponse>().await?;
    //println!("UsersResponse: {:#?}", js);

    let mut users: Vec<UserResult> = Vec::new();
    for (_key, value) in response_data.data {
        // If soldier is not from the PC namespace
        if value.persona.namespace != "cem_ea_id" {
            continue;
        }

        // If soldier has BF4
        for val in value.persona.games.values() {
            if val.is_string() {
                if val.as_str().unwrap().parse::<i32>().unwrap() & 2048 == 0 {
                    continue;
                }
            }
            else if val.is_u64() {
                if val.as_u64().unwrap() & 2048 == 0 {
                    continue;
                }
            }
            else {
                continue;
            }

            users.push(value.clone());
        }
    }

    if !users.is_empty() {
        return Ok(users);
    }

    Err(anyhow::anyhow!("Users not found"))
}

pub async fn get_loadout(soldier_name: &str, persona_id: &str) -> Result<LoadoutResult, anyhow::Error> {
    let client = reqwest::Client::new();
    let res = client
        .get(format!("https://battlelog.battlefield.com/bf4/loadout/get/{}/{}/1/", soldier_name, persona_id))
        .header(USER_AGENT, "BattleFox")
        .send()
        .await?;

    let status = res.status();
    let data_str = res.text().await?;
    // println!("{}", data_str);

    if status != StatusCode::OK {
        return Err(anyhow::anyhow!(data_str));
    }

    let response_data = match serde_json::from_str::<LoadoutResponse>(&data_str) {
        Ok(data) => data,
        Err(err) => return Err(anyhow::anyhow!("{} - {}", err, data_str)),
    };

    // let response_data = res.json::<LoadoutResponse>().await?;
    // println!("LoadoutResponse: {:#?}", response_data);

    if response_data.r#type == "success" && response_data.message == "OK" {
        return Ok(response_data.data);
    }

    Err(anyhow::anyhow!("Loadout not found for {} ({})", soldier_name, persona_id))
}

pub async fn search_user(soldier_name: &str) -> Result<SearchResult, anyhow::Error> {
    let params = [("query", soldier_name.to_owned())];
    let client = reqwest::Client::new();
    let res = client
        .post("https://battlelog.battlefield.com/bf4/search/query/")
        .form(&params)
        .header(USER_AGENT, "BattleFox")
        .send()
        .await?;

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

            return Ok(js.data.remove(i));
        }
    }

    Err(anyhow::anyhow!("User not found"))
}

pub async fn server_snapshot(server_guid: String) -> Result<KeeperResponse, anyhow::Error> {
    let res = reqwest::Client::new()
        .get(format!(
            "https://keeper.battlelog.com/snapshot/{}",
            server_guid
        ))
        .header(USER_AGENT, "BattleFox")
        .send()
        .await?;

    let status = res.status();

    let data_str = res.text().await?;
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
        .get(format!(
            "https://battlelog.battlefield.com/api/bf4/pc/persona/1/{}/ingame_metadata",
            persona_id
        ))
        .header(USER_AGENT, "BattleFox")
        .send()
        .await?;

    let status = res.status();

    let data_str = res.text().await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // Sadly we can't use asserts, since the player may not be in the server actually.
    #[tokio::test]
    async fn get_snapshot() {
        let data = server_snapshot("4d0151b3-81ff-4268-b4e8-5e60d5bc8765".to_string())
            .await
            .unwrap();
        println!("{:#?}", data);

        let player_by_personaid = data.snapshot.get_player_by_personaid(806262072);
        if player_by_personaid.is_some() {
            println!(
                "Found player by personaid: {:#?}",
                player_by_personaid.unwrap()
            );
        }

        let player_by_name = data.snapshot.get_player_by_name("xfileFIN");
        if player_by_name.is_some() {
            println!("Found player by name: {:#?}", player_by_name.unwrap());
        }

        // Uncomment to actually display the output of the println!() statements above:
        // panic!();
    }

    #[tokio::test]
    async fn get_ingame_metadata() {
        let meta = ingame_metadata(806262072).await.unwrap();
        assert_eq!(806262072, meta.persona_id);
    }

    #[tokio::test]
    async fn search_user_test() {
        dbg!(search_user("xfileFIN").await.unwrap());
        // panic!()
    }

    #[tokio::test]
    async fn test_get_users() {
        let soldiers: Vec<String> = vec!["xfileFIN".to_string()];
        dbg!(get_users(soldiers).await.unwrap());
        // panic!()
    }

    #[tokio::test]
    async fn test_get_loadout() {
        dbg!(get_loadout("xfileFIN", "806262072").await.unwrap());
        // panic!()
    }

    #[tokio::test]
    #[ignore]
    async fn test_parse_users() {
        let file = fs::read_to_string("response.json").unwrap();
        let data: UsersResponse = serde_json::from_str(&file).unwrap();

        dbg!(data);
        // panic!()
    }
}
