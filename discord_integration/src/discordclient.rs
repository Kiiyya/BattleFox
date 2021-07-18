use anyhow::Error;
use battlelog::battlelog::{search_user, SearchResult};
use chrono::prelude::*;
use database::{establish_connection, get_battlelog_player_by_persona_id};
use lazy_static::lazy_static;
use serde_json::{json, Value};
use serenity::{async_trait, builder::CreateComponents, client::{Context, EventHandler}, http::{Http, HttpBuilder}, model::{channel::{Embed}, interactions::{ButtonStyle, Interaction}, prelude::Ready, webhook::Webhook}};
use shared::report::ReportModel;

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

    // Channel where admin reports will be sent
    static ref ADMIN_REPORTS_CHANNEL_ID: u64 = dotenv::var("ADMIN_REPORTS_CHANNEL_ID")
        .expect("Expected an admin reports channel id in the environment")
        .parse()
        .expect("admin reports id is not a valid id");

    // Channel where public reports will be sent
    #[derive(Debug)]
    static ref PUBLIC_REPORTS_CHANNEL_ID: u64 = dotenv::var("PUBLIC_REPORTS_CHANNEL_ID")
        .map(|var| var.parse::<u64>())
        .unwrap_or(Ok(0))
        .unwrap();
}

pub struct DiscordClient {
    http: Option<Http>,
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, _ctx: Context, interaction: Interaction) {
        println!("New interaction received: {:#?}", interaction);
    }

    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

impl DiscordClient {
    pub fn new(http: Option<Http>) -> Self {
        Self { http }
    }

    pub async fn run(self: &mut Self) -> Result<(), anyhow::Error> {
        let http = HttpBuilder::new(DISCORD_TOKEN.clone())
            .application_id(*APPLICATION_ID)
            .await
            .expect("Error creating Http");

        *self = Self::new(Some(http));

        // TODO: When there's support for button and select menu interactions, test if kill, kick, ban, mute would be possible from the report
        // // Build our client.
        // let mut client = Client::builder(DISCORD_TOKEN.clone())
        //     .event_handler(Handler)
        //     .application_id(*APPLICATION_ID)
        //     .await
        //     .expect("Error creating client");

        // // Finally, start a single shard, and start listening to events.
        // tokio::spawn( async move { 
        //     if let Err(why) = client.start().await {
        //         println!("Client error: {:?}", why);
        //     }
        // });

        Ok(())
    }

    pub async fn post_report(self: &Self, report: ReportModel) {
        println!("{:#?}", report);

        let reporter = report.reporter.clone();
        let reported = report.reported.clone();

        let issuer = search_user(reporter.to_string()).await;
        let target = search_user(reported.to_string()).await;

        let http = match &self.http {
            Some(http) => http,
            _ => return,
        };

        self.post_admin_report(http, &report, &issuer, &target)
            .await;
        self.post_public_report(http, &report, &issuer, &target)
            .await;
    }

    async fn ensure_webhook(
        self: &Self,
        http: &Http,
        channel_id: u64,
        webhook_name: &str,
    ) -> Option<Webhook> {
        // Ensure webhook has been created, or if not, create it
        let mut webhooks = http.get_channel_webhooks(channel_id).await.unwrap();
        webhooks.retain(|x| {
            x.name
                .as_ref()
                .unwrap_or(&String::from(""))
                .eq(webhook_name)
        });

        let mut webhook: Option<Webhook> = None;
        if webhooks.len() <= 0 {
            let map = json!({ "name": webhook_name });

            match http.create_webhook(channel_id, &map).await {
                Ok(hook) => webhook = Some(hook),
                _ => (),
            }
        } else {
            webhook = Some(webhooks.remove(0));
        }

        return webhook;
    }

    async fn post_admin_report(
        self: &Self,
        http: &Http,
        report: &ReportModel,
        issuer: &Result<SearchResult, Error>,
        target: &Result<SearchResult, Error>,
    ) {
        let webhook = self
            .ensure_webhook(http, *ADMIN_REPORTS_CHANNEL_ID, "battlefox_admin_reports")
            .await;

        let value = self.build_report_message(true, report, issuer, target);
        let map = value.as_object().unwrap();

        // Execute webhook
        match webhook {
            Some(webhook) => {
                println!("{:#?}", webhook.url());
                let _message = http
                    .execute_webhook(webhook.id.0, &webhook.token.unwrap(), true, map)
                    .await
                    .unwrap();
            }
            _ => (),
        }
    }

    async fn post_public_report(
        self: &Self,
        http: &Http,
        report: &ReportModel,
        issuer: &Result<SearchResult, Error>,
        target: &Result<SearchResult, Error>,
    ) {
        if PUBLIC_REPORTS_CHANNEL_ID.eq(&0) {
            return;
        }

        let webhook = self
            .ensure_webhook(http, *PUBLIC_REPORTS_CHANNEL_ID, "battlefox_public_reports")
            .await;

        let value = self.build_report_message(false, report, issuer, target);
        let map = value.as_object().unwrap();

        // Execute webhook
        match webhook {
            Some(webhook) => {
                println!("{:#?}", webhook.url());
                let _message = http
                    .execute_webhook(webhook.id.0, &webhook.token.unwrap(), true, map)
                    .await
                    .unwrap();
            }
            _ => (),
        }
    }

    fn build_report_message(
        self: &Self,
        is_admin: bool,
        report: &ReportModel,
        issuer: &Result<SearchResult, Error>,
        target: &Result<SearchResult, Error>,
    ) -> Value {
        let reporter = &report.reporter;
        let reported = &report.reported;
        let reason = &report.reason;
        let server_name = &report.server_name;
        let server_guid = &report.server_guid;
        let bfacp_url = &report.bfacp_link;

        // Embed building
        let embed = Embed::fake(|e| {
            match server_guid {
                Some(guid) => {
                    e.field(
                        "Server",
                        format!(
                            "[{}](https://battlelog.battlefield.com/bf4/servers/show/pc/{}/)",
                            server_name, guid
                        ),
                        false,
                    );
                }
                None => {
                    e.field("Server", format!("{}", server_name), false);
                }
            }

            // Target thumbnail
            e.thumbnail(match target {
                Ok(user) => user.user.gravatar_md5.clone().map_or(
                    "https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png".to_string(),
                    |md5| format!("https://www.gravatar.com/avatar/{}?d=https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png", md5)
                ),
                _ => "https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png".to_string(),
            });

            // Set title, color, last updated time and footer
            let last_updated_time: DateTime<Utc> = Utc::now();
            e.title(format!("Reported {}", &reported))
                .description(format!("```{}```", reason))
                .colour(0x00ff00)
                .footer(|f| {
                    f.text(format!(
                        "© BattleFox Admin Alerter ({})",
                        last_updated_time.year()
                    ));
                    f
                })
                .timestamp(&last_updated_time);
            e
        });

        // Components
        let mut components = CreateComponents::default();

        match target {
            Ok(user) => {
                // Main links
                components.create_action_row(|r| {
                    r.create_button(|b| {
                        b.label("Battlelog").url(format!("https://battlelog.battlefield.com/bf4/soldier/{0}/stats/{1}/pc/", user.persona_name, user.persona_id)).style(ButtonStyle::Link)
                    })
                    .create_button(|b| {
                        b.label("247fairplay").url(format!("https://www.247fairplay.com/CheatDetector/{0}", user.persona_name)).style(ButtonStyle::Link)
                    })
                    .create_button(|b| {
                        b.label("BF4CR").url(format!("https://bf4cheatreport.com/?pid={0}&uid=&cnt=200&startdate=", user.persona_id)).style(ButtonStyle::Link)
                    })
                    .create_button(|b| {
                        b.label("BF4DB").url(format!("https://www.bf4db.com/player/{0}", user.persona_id)).style(ButtonStyle::Link)
                    });
                    r
                });

                // Admin links
                if is_admin {
                    match bfacp_url {
                        Some(link) => {
                            let connection = establish_connection();
                            let adkats_player = get_battlelog_player_by_persona_id(
                                &connection,
                                &(user.persona_id as i64),
                            );

                            match adkats_player {
                                Ok(player) => {
                                    components.create_action_row(|r| {
                                        r.create_button(|b| {
                                            b.label("BFACP")
                                                .url(format!(
                                                    "{0}/players/{1}/{2}",
                                                    link, player.player_id, user.persona_name
                                                ))
                                                .style(ButtonStyle::Link)
                                        });
                                        r
                                    });
                                }
                                Err(err) => println!("Error fetching adkats_player: {}", err),
                            }
                        }
                        _ => (),
                    }
                }
            }
            _ => {
                // Main links
                components.create_action_row(|r| {
                    r.create_button(|b| {
                        b.label("Battlelog").url(format!("https://battlelog.battlefield.com/bf4/user/{0}/", reported)).style(ButtonStyle::Link)
                    })
                    .create_button(|b| {
                        b.label("247fairplay").url(format!("https://www.247fairplay.com/CheatDetector/{0}", reported)).style(ButtonStyle::Link)
                    })
                    .create_button(|b| {
                        b.label("BF4CR").url(format!("https://bf4cheatreport.com/?pid=&uid={0}&cnt=200&startdate=", reported)).style(ButtonStyle::Link)
                    })
                    .create_button(|b| {
                        b.label("BF4DB").url(format!("https://bf4db.com/player/search?query={0}", reported)).style(ButtonStyle::Link)
                    });
                    r
                });
            },
        };

        json!({
            "username": reporter,
            "avatar_url": match issuer {
                Ok(user) => user.user.gravatar_md5.clone().map_or(
                    "https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png".to_string(),
                    |md5| format!("https://www.gravatar.com/avatar/{}?d=https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png", md5)
                ),
                _ => "https://eaassets-a.akamaihd.net/battlelog/defaultavatars/default-avatar-36.png".to_string(),
            },
            "content": if is_admin { "@here" } else { "" },
            "embeds": [
                embed
            ],
            "components": components.0
        })
    }
}
