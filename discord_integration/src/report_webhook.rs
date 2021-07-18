extern crate battlelog;

use std::num::ParseIntError;

use anyhow::Error;
use chrono::prelude::*;
use database::{establish_connection, get_battlelog_player_by_persona_id};
use serenity::{
    builder::{CreateEmbed},
    http::{Http},
    model::{
        channel::{Embed},
    },
};
use lazy_static::lazy_static;
use battlelog::battlelog::{search_user, SearchResult};

use shared::report::ReportModel;

lazy_static! {
    // Discord webhook id where the admin reports will be sent
    static ref REPORT_WEBHOOK_ID: u64 = dotenv::var("REPORT_WEBHOOK_ID")
        .expect("Expected an report webhook id in the environment")
        .parse()
        .expect("report webhook id is not a valid id");
    //Discord webhook token where the admin reports will be sent
    static ref REPORT_WEBHOOK_TOKEN: String = dotenv::var("REPORT_WEBHOOK_TOKEN")
        .expect("Expected a report webhook token in the environment");

    // Discord webhook id where the public reports will be sent
    #[derive(Debug)]
    static ref PUBLIC_REPORT_WEBHOOK_ID: u64 = dotenv::var("PUBLIC_REPORT_WEBHOOK_ID")
        .map(|var| var.parse::<u64>())
        .unwrap_or(Ok(0))
        .unwrap();

    // // Discord webhook id where the public reports will be sent
    // static ref PUBLIC_REPORT_WEBHOOK_ID: u64 = dotenv::var("PUBLIC_REPORT_WEBHOOK_ID")
    //     .unwrap_or_default()
    //     .parse()
    //     .unwrap_or_default();
    //Discord webhook token where the public reports will be sent
    static ref PUBLIC_REPORT_WEBHOOK_TOKEN: String = dotenv::var("PUBLIC_REPORT_WEBHOOK_TOKEN")
        .unwrap_or_default();
}

pub(crate) async fn report_player_webhook(report: ReportModel) {
    let reporter = report.reporter.clone();
    let reported = report.reported.clone();

    let reporter_user = search_user(reporter.to_string()).await;
    let reported_user = search_user(reported.to_string()).await;

    post_admin_report(&report, &reporter_user, &reported_user).await;
    post_public_report(&report, &reporter_user, &reported_user).await;

    // let http = Http::default();
    // let webhook = http
    //     .get_webhook_with_token(*REPORT_WEBHOOK_ID, &REPORT_WEBHOOK_TOKEN.to_string())
    //     .await
    //     .unwrap();

    // webhook
    //     .execute(&http, false, |w| {

    //         let embed = Embed::fake(|e| {
    //             match server_guid {
    //                 Some(guid) => {
    //                     e.field(
    //                         "Server",
    //                         format!("[{}](https://battlelog.battlefield.com/bf4/servers/show/pc/{}/)", server_name, guid), false);
    //                 },
    //                 None => {
    //                     e.field(
    //                         "Server",
    //                         format!("{}", server_name), false);
    //                 }
    //             }

    //             match (reporter_user, reported_user) {
    //                 (Ok(reporter_data), Ok(reported_data)) => {
    //                     println!("Reporter Persona: {:#?}", reporter_data);
    //                     println!("Reported Persona: {:#?}", reported_data);

    //                     let gravatar_url = reporter_data.user.gravatar_md5.clone().map_or(
    //                         "https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png".to_string(), 
    //                          |md5| format!("https://www.gravatar.com/avatar/{}?d=https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png", md5)
    //                    );

    //                    w.avatar_url(gravatar_url);

    //                    add_reported_to_embed(e, Some(reported_data), bfacp_url);
    //                 }
    //                 (Ok(reporter_data), Err(reported_error)) => {
    //                     println!("Reporter Persona: {:#?}", reporter_data);
    //                     println!("Error fetching reported persona: {:?}", reported_error);

    //                     let gravatar_url = reporter_data.user.gravatar_md5.clone().map_or(
    //                         "https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png".to_string(), 
    //                          |md5| format!("https://www.gravatar.com/avatar/{}?d=https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png", md5)
    //                    );

    //                    w.avatar_url(gravatar_url);

    //                     add_reported_to_embed(e, None, None);
    //                 }
    //                 (Err(reporter_error), Ok(reported_data)) => {
    //                     println!("Error fetching reporter persona: {:?}", reporter_error);
    //                     println!("Reported Persona: {:#?}", reported_data);

    //                     add_reported_to_embed(e, Some(reported_data), bfacp_url);
    //                 }
    //                 (Err(reporter_error), Err(reported_error)) => {
    //                     println!("Error fetching reporter persona: {:?}", reporter_error);
    //                     println!("Error fetching reported persona: {:?}", reported_error);
    //                 }
    //             }

    //             // Set title, color, last updated time and footer
    //             let last_updated_time: DateTime<Utc> = Utc::now();
    //             e.title(format!("Reported {}", &reported))
    //                 .description(reason)
    //                 .colour(0x00ff00)
    //                 .footer(|f| {
    //                     f.text("© BattleFox Admin Alerter (2021)");
    //                     f
    //                 })
    //                 .timestamp(&last_updated_time);
    //             e
    //         });

    //         w.content("@here");
    //         w.username(reporter);
    //         w.embeds(vec![embed])
    //     })
    //     .await
    //     .unwrap();
}

async fn post_admin_report(report: &ReportModel, reporter_user: &Result<SearchResult, Error>, reported_user: &Result<SearchResult, Error>) {
    let reporter = &report.reporter;
    let reported = &report.reported;
    let reason = &report.reason;
    let server_name = &report.server_name;
    let server_guid = &report.server_guid;
    let bfacp_url = &report.bfacp_link;
    
    let http = Http::default();
    let webhook = http
        .get_webhook_with_token(*REPORT_WEBHOOK_ID, &REPORT_WEBHOOK_TOKEN.to_string())
        .await
        .unwrap();

    webhook
        .execute(&http, false, |w| {

            let embed = Embed::fake(|e| {
                match server_guid {
                    Some(guid) => {
                        e.field(
                            "Server",
                            format!("[{}](https://battlelog.battlefield.com/bf4/servers/show/pc/{}/)", server_name, guid), false);
                    },
                    None => {
                        e.field(
                            "Server",
                            format!("{}", server_name), false);
                    }
                }

                match (reporter_user, reported_user) {
                    (Ok(reporter_data), Ok(reported_data)) => {
                        println!("Reporter Persona: {:#?}", reporter_data);
                        println!("Reported Persona: {:#?}", reported_data);

                        let gravatar_url = reporter_data.user.gravatar_md5.clone().map_or(
                            "https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png".to_string(), 
                             |md5| format!("https://www.gravatar.com/avatar/{}?d=https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png", md5)
                       );

                       w.avatar_url(gravatar_url);

                       add_reported_to_embed(e, Some(reported_data), bfacp_url);
                    }
                    (Ok(reporter_data), Err(reported_error)) => {
                        println!("Reporter Persona: {:#?}", reporter_data);
                        println!("Error fetching reported persona: {:?}", reported_error);

                        let gravatar_url = reporter_data.user.gravatar_md5.clone().map_or(
                            "https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png".to_string(), 
                             |md5| format!("https://www.gravatar.com/avatar/{}?d=https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png", md5)
                       );

                       w.avatar_url(gravatar_url);

                        add_reported_to_embed(e, None, &None);
                    }
                    (Err(reporter_error), Ok(reported_data)) => {
                        println!("Error fetching reporter persona: {:?}", reporter_error);
                        println!("Reported Persona: {:#?}", reported_data);

                        add_reported_to_embed(e, Some(reported_data), bfacp_url);
                    }
                    (Err(reporter_error), Err(reported_error)) => {
                        println!("Error fetching reporter persona: {:?}", reporter_error);
                        println!("Error fetching reported persona: {:?}", reported_error);
                    }
                }

                // Set title, color, last updated time and footer
                let last_updated_time: DateTime<Utc> = Utc::now();
                e.title(format!("Reported {}", &reported))
                    .description(format!("```{}```", reason))
                    .colour(0x00ff00)
                    .footer(|f| {
                        f.text(format!("© BattleFox Admin Alerter ({})", last_updated_time.year()));
                        f
                    })
                    .timestamp(&last_updated_time);
                e
            });

            w.content("@here");
            w.username(reporter);
            w.embeds(vec![embed])
        })
        .await
        .unwrap();
}

async fn post_public_report(report: &ReportModel, reporter_user: &Result<SearchResult, Error>, reported_user: &Result<SearchResult, Error>) {
    println!("Public webhook id: {:?}", PUBLIC_REPORT_WEBHOOK_ID);
    
    if PUBLIC_REPORT_WEBHOOK_ID.eq(&0) {
        return;
    }

    let reporter = &report.reporter;
    let reported = &report.reported;
    let reason = &report.reason;
    let server_name = &report.server_name;
    let server_guid = &report.server_guid;
    
    let http = Http::default();
    let webhook = http
        .get_webhook_with_token(*REPORT_WEBHOOK_ID, &REPORT_WEBHOOK_TOKEN.to_string())
        .await
        .unwrap();

    webhook
        .execute(&http, false, |w| {

            let embed = Embed::fake(|e| {
                match server_guid {
                    Some(guid) => {
                        e.field(
                            "Server",
                            format!("[{}](https://battlelog.battlefield.com/bf4/servers/show/pc/{}/)", server_name, guid), false);
                    },
                    None => {
                        e.field(
                            "Server",
                            format!("{}", server_name), false);
                    }
                }

                match (reporter_user, reported_user) {
                    (Ok(reporter_data), Ok(reported_data)) => {
                        println!("Reporter Persona: {:#?}", reporter_data);
                        println!("Reported Persona: {:#?}", reported_data);

                        let gravatar_url = reporter_data.user.gravatar_md5.clone().map_or(
                            "https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png".to_string(), 
                             |md5| format!("https://www.gravatar.com/avatar/{}?d=https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png", md5)
                       );

                       w.avatar_url(gravatar_url);

                       add_reported_to_embed(e, Some(reported_data), &None);
                    }
                    (Ok(reporter_data), Err(reported_error)) => {
                        println!("Reporter Persona: {:#?}", reporter_data);
                        println!("Error fetching reported persona: {:?}", reported_error);

                        let gravatar_url = reporter_data.user.gravatar_md5.clone().map_or(
                            "https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png".to_string(), 
                             |md5| format!("https://www.gravatar.com/avatar/{}?d=https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png", md5)
                       );

                       w.avatar_url(gravatar_url);

                        add_reported_to_embed(e, None, &None);
                    }
                    (Err(reporter_error), Ok(reported_data)) => {
                        println!("Error fetching reporter persona: {:?}", reporter_error);
                        println!("Reported Persona: {:#?}", reported_data);

                        add_reported_to_embed(e, Some(reported_data), &None);
                    }
                    (Err(reporter_error), Err(reported_error)) => {
                        println!("Error fetching reporter persona: {:?}", reporter_error);
                        println!("Error fetching reported persona: {:?}", reported_error);
                    }
                }

                // Set title, color, last updated time and footer
                let last_updated_time: DateTime<Utc> = Utc::now();
                e.title(format!("Reported {}", &reported))
                    .description(reason)
                    .colour(0x00ff00)
                    .footer(|f| {
                        f.text("© BattleFox Admin Alerter (2021)");
                        f
                    })
                    .timestamp(&last_updated_time);
                e
            });

            w.content("");
            w.username(reporter);
            w.embeds(vec![embed])
        })
        .await
        .unwrap();
}

fn add_reported_to_embed<'a>(
    embed: &'a mut CreateEmbed,
    reported: Option<&SearchResult>,
    bfacp_link: &Option<String>
) -> &'a mut CreateEmbed {
    match reported {
        Some(user) => {
            let gravatar_url = user.user.gravatar_md5
                .clone().map_or(
            "https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png".to_string(), 
            |md5| format!("https://www.gravatar.com/avatar/{}?d=https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png", md5)
            );

            embed
                .thumbnail(gravatar_url)
                // .field(
                //     "Links",
                //     format!(
                //         "[Battlelog](https://battlelog.battlefield.com/bf4/soldier/{0}/stats/{1}/pc/)\n\n[247fairplay](https://www.247fairplay.com/CheatDetector/{0})\n\n[BF4CR](https://bf4cheatreport.com/?pid={1}&uid=&cnt=200&startdate=)\n\n[BF4DB](https://www.bf4db.com/player/{1})",
                //         user.persona_name, user.persona_id
                //     ),
                //     true,
                // )
                .field(
                    "Links",
                    format!(
                        "[Battlelog](https://battlelog.battlefield.com/bf4/soldier/{0}/stats/{1}/pc/)",
                        user.persona_name, user.persona_id
                    ),
                    true,
                )
                .field(
                    "\n\u{200b}",
                    format!(
                        "[247fairplay](https://www.247fairplay.com/CheatDetector/{0})",
                        user.persona_name
                    ),
                    true,
                )
                .field(
                    "\n\u{200b}",
                    format!(
                        "[BF4CR](https://bf4cheatreport.com/?pid={0}&uid=&cnt=200&startdate=)",
                        user.persona_id
                    ),
                    true,
                )
                .field(
                    "\n\u{200b}",
                    format!(
                        "[BF4DB](https://www.bf4db.com/player/{0})",
                        user.persona_id
                    ),
                    true,
                );

                match bfacp_link {
                    Some(link) => {
                        let connection = establish_connection();
                        let adkats_player = get_battlelog_player_by_persona_id(&connection, &(user.persona_id as i64));
            
                        match adkats_player {
                            Ok(player) => {
                                embed
                                    .field(
                                        "Admin links",
                                        format!(
                                            "[BFACP]({0}/players/{1}/{2})",
                                            link, player.player_id, user.persona_name
                                        ),
                                        false,
                                    );
                            },
                            Err(err) => println!("Error fetching adkats_player: {}", err),
                        }
                    },
                    _ => (),
                }
                

            return embed;
        }
        None => {
            return embed.thumbnail(
                "https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png"
                    .to_string(),
            );
        }
    }
}
