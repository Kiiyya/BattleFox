#[path = "models/report.rs"] mod report;
#[path = "report_webhook.rs"] mod report_webhook;

use futures::{StreamExt, future::join};
use lapin::{BasicProperties, Connection, ConnectionProperties, options::{BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions}, types::FieldTable};
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
    let _queue = channel.queue_declare("bf4_reports", QueueDeclareOptions::default(), FieldTable::default()).await?;

    // Start a consumer.
    let mut consumer = channel.basic_consume("bf4_reports", "reports_consumer", BasicConsumeOptions::default(), FieldTable::default()).await?;
    let consumer_joinhandle = tokio::spawn(async move { // the `move` will move the consumer *into* the task.
        info!("Waiting for consume...");
        while let Some(delivery) = consumer.next().await {
            info!("Got something! Acking...");
            let (channel, delivery) = delivery.expect("error in consumer");

            let body = String::from_utf8_lossy(&delivery.data);
            let report = serde_json::from_str::<report::ReportModel>(&body).unwrap();
            println!("Received [{:?}]", report);

            report_webhook::report_player_webhook(report).await;

            delivery
                .ack(BasicAckOptions::default()).await
                .expect("ack failed");
            info!("Acknowledged.");
        }
        debug!("Consumer loop ended gracefully");
    });
    // println!("Waiting for messages. Press Ctrl-C to exit.");

    // for (i, message) in consumer.receiver().iter().enumerate() {
    //     match message {
    //         ConsumerMessage::Delivery(delivery) => {
    //             let body = String::from_utf8_lossy(&delivery.body);
    //             let report = serde_json::from_str::<report::ReportModel>(&body).unwrap();
    //             println!("({:>3}) Received [{:?}]", i, report);

    //             report_webhook::report_player_webhook(report).await;

    //             //let _ = Runtime::new().unwrap().block_on(report_webhook::report_player_webhook(report));
    //             consumer.ack(delivery)?;
    //         }
    //         other => {
    //             println!("Consumer ended: {:?}", other);
    //             break;
    //         }
    //     }
    // }

    consumer_joinhandle.await?; // wait for our consumer to quit.

    Ok(())
}
