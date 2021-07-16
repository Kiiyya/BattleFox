extern crate battlelog;
#[macro_use] extern crate log;

mod rabbitmq;

use chrono::prelude::*;
use dotenv::dotenv;
use log::LevelFilter;
use serenity::{
    async_trait,
    builder::{CreateEmbed, ExecuteWebhook},
    http::{client::HttpBuilder, Http},
    model::{
        channel::{Embed, Message},
        gateway::Ready,
        id::ChannelId,
    },
    prelude::*,
};
use lazy_static::lazy_static;
use battlelog::battlelog::{search_user, SearchResult};
use simplelog::{Config, SimpleLogger};
use std::thread;
use tokio::runtime::Runtime;

struct Handler;

lazy_static! {
    // Configure the client with your Discord bot token in the environment.
    static ref DISCORD_TOKEN: String = dotenv::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    // The Application Id is usually the Bot User Id.
    static ref APPLICATION_ID: u64 = dotenv::var("APPLICATION_ID")
        .expect("Expected an application id in the environment")
        .parse()
        .expect("application id is not a valid id");
    // Discord channel id where the messages will be sent
    static ref LOG_CHANNEL_ID: u64 = dotenv::var("LOG_CHANNEL_ID")
        .expect("Expected an log channel id in the environment")
        .parse()
        .expect("log channel id is not a valid id");
    // Discord webhook id where the reports will be sent
    static ref REPORT_WEBHOOK_ID: u64 = dotenv::var("REPORT_WEBHOOK_ID")
        .expect("Expected an report webhook id in the environment")
        .parse()
        .expect("report webhook id is not a valid id");
    //Discord webhook token where the reports will be sent
    static ref REPORT_WEBHOOK_TOKEN: String = dotenv::var("REPORT_WEBHOOK_TOKEN").expect("Expected a report webhook token in the environment");
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, context: Context, msg: Message) {
        if msg.content == "!ping" {
            if let Err(why) = msg.channel_id.say(&context.http, "Pong!").await {
                println!("Error sending message: {}", why);
            }
        }
    }

    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let _report = report_player_webhook(
            "PocketWolfy".to_string(),
            "xfileFIN".to_string(),
            // "PocketWolfy".to_string(),
            // "abcdefg".to_string(),
            //"abcdefg".to_string(),
            //"xfileFIN".to_string(),
            "He's a noob".to_string(),
            "!! LSD !! HARDCORE RUSH | NO STUPID RULES | NO MORTAR/UCAV".to_string(),
            "4d0151b3-81ff-4268-b4e8-5e60d5bc8765".to_string()
        )
        .await;
    }
}

async fn report_player_webhook(reporter: String, reported: String, reason: String, server_name: String, server_guid: String) {
    let reporter_user = search_user(reporter.to_string()).await;
    let reported_user = search_user(reported.to_string()).await;

    let http = Http::default();
    let webhook = http
        .get_webhook_with_token(*REPORT_WEBHOOK_ID, &REPORT_WEBHOOK_TOKEN.to_string())
        .await
        .unwrap();

    webhook
        .execute(&http, false, |w| {

            let embed = Embed::fake(|e| {
                e.field(
                "Server",
                format!("[{}](https://battlelog.battlefield.com/bf4/servers/show/pc/{}/)", server_name, server_guid), false);

                match (reporter_user, reported_user) {
                    (Ok(reporter_data), Ok(reported_data)) => {
                        println!("Reporter Persona: {:#?}", reporter_data);
                        println!("Reported Persona: {:#?}", reported_data);

                        let gravatar_url = reporter_data.user.gravatar_md5.clone().map_or(
                            "https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png".to_string(),
                             |md5| format!("https://www.gravatar.com/avatar/{}?d=https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png", md5)
                       );

                       w.avatar_url(gravatar_url);

                       add_reported_to_embed(e, Some(reported_data));
                    }
                    (Ok(reporter_data), Err(reported_error)) => {
                        println!("Reporter Persona: {:#?}", reporter_data);
                        println!("Error fetching reported persona: {:?}", reported_error);

                        let gravatar_url = reporter_data.user.gravatar_md5.clone().map_or(
                            "https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png".to_string(),
                             |md5| format!("https://www.gravatar.com/avatar/{}?d=https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png", md5)
                       );

                       w.avatar_url(gravatar_url);

                        add_reported_to_embed(e, None);
                    }
                    (Err(reporter_error), Ok(reported_data)) => {
                        println!("Error fetching reporter persona: {:?}", reporter_error);
                        println!("Reported Persona: {:#?}", reported_data);

                        add_reported_to_embed(e, Some(reported_data));
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

            w.content("@here");
            w.username(reporter);
            w.embeds(vec![embed])
        })
        .await
        .unwrap();
}

async fn report_player(reporter: String, reported: String, reason: String) {
    let http = HttpBuilder::new(DISCORD_TOKEN.clone())
        .application_id(*APPLICATION_ID)
        .await
        .expect("Error creating Http");
    let channel = ChannelId(*LOG_CHANNEL_ID);

    let reporter_user = search_user(reporter.to_string()).await;
    let reported_user = search_user(reported.to_string()).await;

    let _ = channel
        .send_message(http, |m| {
            m.content(format!(
                "@here **{}** reported **{}** for {}",
                reporter, reported, reason
            ));

            m.embed(|e| {
                match (reporter_user, reported_user) {
                    (Ok(reporter_data), Ok(reported_data)) => {
                        println!("Reporter Persona: {:#?}", reporter_data);
                        println!("Reported Persona: {:#?}", reported_data);

                        add_reporter_to_embed(e, Some(reporter_data), reporter);
                        add_reported_to_embed(e, Some(reported_data));
                    }
                    (Ok(reporter_data), Err(reported_error)) => {
                        println!("Reporter Persona: {:#?}", reporter_data);
                        println!("Error fetching reported persona: {:?}", reported_error);

                        add_reporter_to_embed(e, Some(reporter_data), reporter);
                        add_reported_to_embed(e, None);
                    }
                    (Err(reporter_error), Ok(reported_data)) => {
                        println!("Error fetching reporter persona: {:?}", reporter_error);
                        println!("Reported Persona: {:#?}", reported_data);

                        add_reporter_to_embed(e, None, reporter);
                        add_reported_to_embed(e, Some(reported_data));
                    }
                    (Err(reporter_error), Err(reported_error)) => {
                        println!("Error fetching reporter persona: {:?}", reporter_error);
                        println!("Error fetching reported persona: {:?}", reported_error);
                    }
                }

                // Set title, color, last updated time and footer
                let last_updated_time: DateTime<Utc> = Utc::now();
                e.title(format!("Report reason: {}", reason))
                    .colour(0x00ff00)
                    .footer(|f| {
                        f.text("© BattleFox Admin Alerter (2021)");
                        f
                    })
                    .timestamp(&last_updated_time)
            })
        })
        .await;
}

fn add_reported_to_embed(
    embed: &mut CreateEmbed,
    reported: Option<SearchResult>,
) -> &mut CreateEmbed {
    match reported {
        Some(user) => {
            let gravatar_url = user.user.gravatar_md5
                .clone().map_or(
            "https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png".to_string(),
            |md5| format!("https://www.gravatar.com/avatar/{}?d=https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png", md5)
            );

            return embed
                .thumbnail(gravatar_url)
                .field(
                    "Links",
                    format!(
                        "[Battlelog](https://battlelog.battlefield.com/bf4/soldier/{0}/stats/{1}/pc/)\n\n[247fairplay](https://www.247fairplay.com/CheatDetector/{0})\n\n[BF4CR](https://bf4cheatreport.com/?pid={1}&uid=&cnt=200&startdate=)\n\n[BF4DB](https://www.bf4db.com/player/{1})",
                        user.persona_name, user.persona_id
                    ),
                    true,
                );

            // return embed
            //     .thumbnail(gravatar_url)
            //     .field(
            //         "Battlelog",
            //         format!(
            //             "[{0} - Battlelog](https://battlelog.battlefield.com/bf4/soldier/{0}/stats/{1}/pc/)",
            //             user.persona_name, user.persona_id
            //         ),
            //         true,
            //     )
            //     .field(
            //         "247fairplay",
            //         format!(
            //             "[{} - 247fairplay](https://www.247fairplay.com/CheatDetector/{})",
            //             user.persona_name, user.persona_name
            //         ),
            //         true,
            //     )
            //     .field(
            //         "BF4CR",
            //         format!(
            //             "[{} - BF4CR](https://bf4cheatreport.com/?pid={}&uid=&cnt=200&startdate=)",
            //             user.persona_name, user.persona_id
            //         ),
            //         true,
            //     )
            //     .field(
            //         "BF4DB",
            //         format!(
            //             "[{} - BF4DB](https://www.bf4db.com/player/{})",
            //             user.persona_name, user.persona_id
            //         ),
            //         true,
            //     );
        }
        None => {
            return embed.thumbnail(
                "https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png"
                    .to_string(),
            );
        }
    }
}

fn add_reporter_to_embed(
    embed: &mut CreateEmbed,
    reporter: Option<SearchResult>,
    reporter_name: String,
) -> &mut CreateEmbed {
    match reporter {
        Some(user) => {
            let gravatar_url = user.user.gravatar_md5.clone().map_or("https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png".to_string(), |md5| format!("https://www.gravatar.com/avatar/{}?d=https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png", md5));

            return embed.author(|f| {
                f.icon_url(gravatar_url)
                    .name(user.persona_name.clone())
                    .url(format!(
                        "https://battlelog.battlefield.com/bf4/soldier/{}/stats/{}/pc/",
                        user.persona_name, user.persona_id
                    ))
            });
        }
        None => {
            return embed.author(|f| {
                f.icon_url("https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png".to_string())
                    .name(reporter_name.clone())
                    .url(format!(
                        "https://battlelog.battlefield.com/bf4/soldier/{}/",
                        reporter_name
                    ))
            });
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let _ = SimpleLogger::init(LevelFilter::Info, Config::default());

    tokio::spawn( async { rabbitmq::initialize_report_consumer().await; });

    // thread::spawn(|| {
    //     let _ = Runtime::new().unwrap().block_on(rabbitmq::initialize_report_consumer());
    // });

    let mut client = Client::builder(DISCORD_TOKEN.clone())
        .event_handler(Handler)
        .application_id(*APPLICATION_ID)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
