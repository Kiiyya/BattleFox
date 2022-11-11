extern crate battlelog;
#[macro_use] extern crate log;

mod discordclient;
mod rabbitmq;

use battlefox_database::BfoxContext;
use discordclient::DiscordClient;
use dotenv::dotenv;
use log::LevelFilter;
use serenity::{async_trait, model::{channel::{Message}, gateway::Ready}, prelude::*};
use lazy_static::lazy_static;
use simplelog::{Config, SimpleLogger};

struct Handler;

lazy_static! {
    // Configure the client with your Discord bot token in the environment.
    static ref DISCORD_TOKEN: String = dotenv::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    // The Application Id is usually the Bot User Id.
    static ref APPLICATION_ID: u64 = dotenv::var("APPLICATION_ID")
        .expect("Expected an application id in the environment")
        .parse()
        .expect("application id is not a valid id");
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
    }
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let _ = SimpleLogger::init(LevelFilter::Warn, Config::default());

    let ctx = BfoxContext::new_env();

    // Report processing
    //tokio::spawn( async { rabbitmq::initialize_report_consumer().await.unwrap(); });

    let mut discord_client = DiscordClient::new(None, ctx);

    if let Err(why) = discord_client.run().await {
        println!("Error running discord client: {:?}", why);
    }

    if let Err(why) = rabbitmq::initialize_report_consumer(discord_client).await {
        println!("Unable to initialize report consumer: {:?}", why);
    }

    // Discord client init
    // let mut client = Client::builder(DISCORD_TOKEN.clone())
    //     .event_handler(Handler)
    //     .application_id(*APPLICATION_ID)
    //     .await
    //     .expect("Err creating client");

    // if let Err(why) = client.start().await {
    //     println!("Client error: {:?}", why);
    // }

    Ok(())
}
