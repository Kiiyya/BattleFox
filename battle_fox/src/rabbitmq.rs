use std::sync::Arc;

use shared::report::ReportModel;
use futures::{StreamExt};
use lapin::{Connection, ConnectionProperties, options::{BasicAckOptions, BasicConsumeOptions, QueueDeclareOptions}, types::FieldTable};
use lazy_static::lazy_static;

lazy_static! {
    // Configure the client with your Discord bot token in the environment.
    static ref RABBITMQ_USERNAME: String = dotenv::var("RABBITMQ_USERNAME").expect("Expected a RabbitMQ username in the environment");
    static ref RABBITMQ_PASSWORD: String = dotenv::var("RABBITMQ_PASSWORD").expect("Expected a RabbitMQ password in the environment");
    // The Application Id is usually the Bot User Id.
    static ref RABBITMQ_HOST: String = dotenv::var("RABBITMQ_HOST").expect("Expected a RabbitMQ host in the environment");
}

pub(crate) async fn initialize_report_consumer() -> Result<(), anyhow::Error> {
    // Open connection.
    let connection = Connection::connect(
        &format!("amqp://{}:{}@{}", RABBITMQ_USERNAME.to_string(), RABBITMQ_PASSWORD.to_string(), RABBITMQ_HOST.to_string()),
        ConnectionProperties::default()
    ).await?;

    // Open a channel - None says let the library choose the channel ID.
    let channel = connection.create_channel().await?;

    // Declare the "bf4_reports" queue.
    let _queue = channel.queue_declare("bfox_main", QueueDeclareOptions::default(), FieldTable::default()).await?;

    // Start a consumer.
    let mut consumer = channel.basic_consume("bfox_main", "bfox_consumer", BasicConsumeOptions::default(), FieldTable::default()).await?;
    let client = Arc::new(client);
    let consumer_joinhandle = tokio::spawn(async move { // the `move` will move the consumer *into* the task.
        info!("Waiting for consume...");
        while let Some(delivery) = consumer.next().await {
            info!("Got something! Acking...");
            let (_channel, delivery) = delivery.expect("error in consumer");

            let body = String::from_utf8_lossy(&delivery.data);
            let report = serde_json::from_str::<ReportModel>(&body).unwrap();
            println!("Received [{:?}]", report);

            let client_clone = client.clone();
            tokio::spawn(async move {

                delivery
                    .ack(BasicAckOptions::default()).await
                    .expect("ack failed");
                info!("Acknowledged.");
            });
        }
        debug!("Consumer loop ended gracefully");
    });

    consumer_joinhandle.await?; // wait for our consumer to quit.

    Ok(())
}
