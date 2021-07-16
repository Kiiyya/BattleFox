#[path = "models/report.rs"] mod report;
#[path = "report_webhook.rs"] mod report_webhook;

use amiquip::{Connection, ConsumerMessage, ConsumerOptions, QueueDeclareOptions, Result};
//use tokio::runtime::Runtime;
use lazy_static::lazy_static;

lazy_static! {
    // Configure the client with your Discord bot token in the environment.
    static ref RABBITMQ_USERNAME: String = dotenv::var("RABBITMQ_USERNAME").expect("Expected a RabbitMQ username in the environment");
    static ref RABBITMQ_PASSWORD: String = dotenv::var("RABBITMQ_PASSWORD").expect("Expected a RabbitMQ password in the environment");
    // The Application Id is usually the Bot User Id.
    static ref RABBITMQ_HOST: String = dotenv::var("RABBITMQ_HOST").expect("Expected a RabbitMQ host in the environment");
}

pub(crate) async fn initialize_report_consumer() -> Result<()> {
    // Open connection.
    let mut connection = Connection::insecure_open(&format!("amqp://{}:{}@{}", RABBITMQ_USERNAME.to_string(), RABBITMQ_PASSWORD.to_string(), RABBITMQ_HOST.to_string()).to_string())?;

    // Open a channel - None says let the library choose the channel ID.
    let channel = connection.open_channel(None)?;

    // Declare the "bf4_reports" queue.
    let queue = channel.queue_declare("bf4_reports", QueueDeclareOptions::default())?;

    // Start a consumer.
    let consumer = queue.consume(ConsumerOptions::default())?;
    println!("Waiting for messages. Press Ctrl-C to exit.");

    for (i, message) in consumer.receiver().iter().enumerate() {
        match message {
            ConsumerMessage::Delivery(delivery) => {
                let body = String::from_utf8_lossy(&delivery.body);
                let report = serde_json::from_str::<report::ReportModel>(&body).unwrap();
                println!("({:>3}) Received [{:?}]", i, report);

                report_webhook::report_player_webhook(report).await;

                //let _ = Runtime::new().unwrap().block_on(report_webhook::report_player_webhook(report));
                consumer.ack(delivery)?;
            }
            other => {
                println!("Consumer ended: {:?}", other);
                break;
            }
        }
    }

    connection.close()
}
