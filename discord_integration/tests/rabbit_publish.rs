//! Helper functions for manual testing. Post
//!
//! Less of a test, and more just a handy function to run manually in order to test manually.
//! Have RabbitMQ running in docker with the following config:
//! ```yml
//! version: '3.2'
//!
//! services:
//! rabbitmq:
//!   image: rabbitmq:3-management
//!   ports:
//!     - "16672:15672"
//!     - "6672:5672"
//!   environment:
//!     - RABBITMQ_DEFAULT_USER=DefaultUser
//!     - RABBITMQ_DEFAULT_PASS=DefaulPassword
//! ```
//!
//! You also need a discord bot, tho for now the access data for the bot is simply hardcoded.

use lapin::{BasicProperties, Connection, ConnectionProperties, options::{BasicPublishOptions, QueueDeclareOptions}, types::FieldTable};
use lazy_static::lazy_static;
use shared::report::ReportModel;

lazy_static! {
    static ref RABBITMQ_USERNAME: String = dotenv::var("RABBITMQ_USERNAME").unwrap_or_else(|_| "DefaultUser".to_string());
    static ref RABBITMQ_PASSWORD: String = dotenv::var("RABBITMQ_PASSWORD").unwrap_or_else(|_| "DefaultPassword".to_string());
    static ref RABBITMQ_HOST: String = dotenv::var("RABBITMQ_HOST").unwrap_or_else(|_| "localhost:6672".to_string());
}

// fn main() -> Result<()> {
//     // Open connection.
//     let mut connection = Connection::insecure_open(&format!("amqp://{}:{}@{}", RABBITMQ_USERNAME.to_string(), RABBITMQ_PASSWORD.to_string(), RABBITMQ_HOST.to_string()).to_string())?;

//     // Open a channel - None says let the library choose the channel ID.
//     let channel = connection.open_channel(None)?;

//     // Get a handle to the direct exchange on our channel.
//     let exchange = Exchange::direct(&channel);

//     // Publish a message to the "bf4_reports" queue.
//     let report = ReportModel {
//         reporter: "PocketWolfy".to_string(),
//         reported: "xfileFIN".to_string(),
//         reason: "Just testing, you know...".to_string(),
//         server_name: "Test server".to_string(),
//         server_guid: "4d0151b3-81ff-4268-b4e8-5e60d5bc8765".to_string()
//     };
//     exchange.publish(Publish::new(serde_json::to_string(&report).unwrap().as_bytes(), "bf4_reports"))?;

//     connection.close()
// }

#[ignore]
#[tokio::test]
async fn rabbit_publish_test() -> Result<(), anyhow::Error> {
    // Open connection.
    let connection = Connection::connect(
        &format!("amqp://{}:{}@{}", RABBITMQ_USERNAME.to_string(), RABBITMQ_PASSWORD.to_string(), RABBITMQ_HOST.to_string()),
        ConnectionProperties::default()
    ).await?;

    // Open a channel - None says let the library choose the channel ID.
    let channel = connection.create_channel().await?;

    // Declare the "bf4_reports" queue.
    let _queue = channel.queue_declare("bf4_reports", QueueDeclareOptions::default(), FieldTable::default()).await?;

    // Publish a message to the "bf4_reports" queue.
    let report = ReportModel {
        reporter: "PocketWolfy".to_string(),
        reported: "xfileFIN".to_string(),
        reason: "Just testing, you know...".to_string(),
        server_name: "Test server with a long server name to see if embed gets wider..!! ??".to_string(),
        server_guid: Some("4d0151b3-81ff-4268-b4e8-5e60d5bc8765".to_string()),
        bfacp_link: Some("https://bfadmin.somebogussite.com".to_string())
    };
    // exchange.publish(Publish::new(serde_json::to_string(&report).unwrap().as_bytes(), "bf4_reports"))?;

    let _confirm = channel
            .basic_publish(
                "",
                "bf4_reports",
                BasicPublishOptions::default(),
                serde_json::to_string(&report).unwrap().as_bytes().to_vec(),
                BasicProperties::default(),
            )
            .await
            .expect("basic_publish")
            .await // Wait for this specific ack/nack
            .expect("publisher-confirms");

    Ok(())
}
