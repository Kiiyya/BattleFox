use lapin::{BasicProperties, Channel, Connection, ConnectionProperties, options::{BasicPublishOptions, QueueDeclareOptions}, types::FieldTable};
use lazy_static::lazy_static;

use crate::report::ReportModel;

lazy_static! {
    static ref RABBITMQ_USERNAME: String = dotenv::var("RABBITMQ_USERNAME").expect("Expected a RabbitMQ username in the environment");
    static ref RABBITMQ_PASSWORD: String = dotenv::var("RABBITMQ_PASSWORD").expect("Expected a RabbitMQ password in the environment");
    static ref RABBITMQ_HOST: String = dotenv::var("RABBITMQ_HOST").expect("Expected a RabbitMQ host in the environment");
}

pub struct RabbitMq {
    channel: Option<Channel>,
}

impl RabbitMq {
    pub fn new(channel: Option<Channel>) -> Self {
        Self {
            channel: channel
        }
    }

    pub async fn run(self: &mut Self) -> Result<(), anyhow::Error> {
        // Open connection
        // let connection = Connection::connect(
        //     &format!("amqp://{}:{}@{}", RABBITMQ_USERNAME.to_string(), RABBITMQ_PASSWORD.to_string(), RABBITMQ_HOST.to_string()),
        //     ConnectionProperties::default()
        // ).await;

        let connection = match Connection::connect(
            &format!("amqp://{}:{}@{}", RABBITMQ_USERNAME.to_string(), RABBITMQ_PASSWORD.to_string(), RABBITMQ_HOST.to_string()),
            ConnectionProperties::default()
        ).await {
            Ok(file) => file,
            Err(error) => return Err(anyhow::anyhow!("Problem connecting to RabbitMQ: {:?}", error)),
        };

        // Open a channel - None says let the library choose the channel ID.
        let channel = connection.create_channel().await.unwrap();

        *self = Self::new({
            Some(channel)
        });

        // Declare the "bf4_reports" queue.
        let _queue = self.channel.as_ref().unwrap().queue_declare("bf4_reports", QueueDeclareOptions::default(), FieldTable::default()).await.unwrap();

        Ok(())
    }

    pub async fn queue_report(self: &Self, report: ReportModel) -> Result<(), anyhow::Error> {
        // Publish a message to the "bf4_reports" queue.
        match &self.channel {
            Some(channel) => {
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
            },
            None => Err(anyhow::anyhow!("RabbitMq channel hasn't been created. Did you forget to call run?")),
        } 
    }
}