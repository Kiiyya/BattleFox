use amiquip::{Connection, Exchange, Publish, Result};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

lazy_static! {
    // Configure the client with your Discord bot token in the environment.
    static ref RABBITMQ_USERNAME: String = dotenv::var("RABBITMQ_USERNAME").expect("Expected a RabbitMQ username in the environment");
    static ref RABBITMQ_PASSWORD: String = dotenv::var("RABBITMQ_PASSWORD").expect("Expected a RabbitMQ password in the environment");
    // The Application Id is usually the Bot User Id.
    static ref RABBITMQ_HOST: String = dotenv::var("RABBITMQ_HOST").expect("Expected a RabbitMQ host in the environment");
}

#[derive(Debug, Deserialize, Serialize)]
struct ReportModel {
    reporter: String,
    reported: String,
    reason: String,
    server_name: String,
    server_guid: String,
}

fn main() -> Result<()> {
    // Open connection.
    let mut connection = Connection::insecure_open(&format!("amqp://{}:{}@{}", RABBITMQ_USERNAME.to_string(), RABBITMQ_PASSWORD.to_string(), RABBITMQ_HOST.to_string()).to_string())?;

    // Open a channel - None says let the library choose the channel ID.
    let channel = connection.open_channel(None)?;

    // Get a handle to the direct exchange on our channel.
    let exchange = Exchange::direct(&channel);

    // Publish a message to the "bf4_reports" queue.
    let report = ReportModel { 
        reporter: "PocketWolfy".to_string(),
        reported: "xfileFIN".to_string(),
        reason: "Just testing, you know...".to_string(),
        server_name: "Test server".to_string(),
        server_guid: "4d0151b3-81ff-4268-b4e8-5e60d5bc8765".to_string()
    };
    exchange.publish(Publish::new(serde_json::to_string(&report).unwrap().as_bytes(), "bf4_reports"))?;

    connection.close()
}