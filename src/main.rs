use async_trait::async_trait;
use log::{info, LevelFilter};
use std::collections::HashMap;

use serde_json::Value;
use std::thread;
use std::time::Duration;

use serenity::model::channel::Message;

use serenity::model::gateway::Ready;

use crate::config::Settings;
use serenity::prelude::*;
use serenity::utils::hashmap_to_json_map;

mod config;
mod date;

struct Handler {
    pub settings: Settings,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, new_message: Message) {
        info!("Message sent: {new_message:?}")
    }

    async fn ready(&self, ctx: Context, data_about_bot: Ready) {
        info!("Bot is ready. id: {}", data_about_bot.user.id);

        let settings = self.settings.clone();
        tokio::spawn(async move {
            let mut current_week_day = String::new();

            loop {
                let new_week_day = date::current_week_day();

                if current_week_day != new_week_day {
                    current_week_day = new_week_day;

                    let mut map = HashMap::new();
                    map.insert(
                        "name".to_owned(),
                        Value::from(format!("Endlich {current_week_day}!")),
                    );

                    ctx.http
                        .edit_channel(settings.day_channel_id, &hashmap_to_json_map(map))
                        .await
                        .expect("failed to rename channel");

                    info!("Renamed channel. Endlich {current_week_day}!");
                }

                thread::sleep(Duration::from_secs(1));
            }
        });
    }
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_module("schilliger_bot", LevelFilter::Info)
        .init();

    let settings = config::load_settings().unwrap_or_default();
    let mut client = Client::builder(&settings.token)
        .event_handler(Handler { settings })
        .await
        .expect("Failed to create client");

    if let Err(cause) = client.start().await {
        eprintln!("Starting client caused error: {:#?}", cause);
    }
}
