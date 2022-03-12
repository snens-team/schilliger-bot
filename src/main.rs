use async_trait::async_trait;
use log::{info, LevelFilter};
use std::collections::HashMap;
use std::iter::Cycle;
use std::sync::Arc;

use rand::prelude::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;
use serde_json::Value;
use std::thread;
use std::time::Duration;

use serenity::model::channel::Message;

use serenity::model::gateway::{Activity, Ready};
use serenity::model::prelude::OnlineStatus;

use crate::config::Settings;
use serenity::prelude::*;
use serenity::utils::hashmap_to_json_map;

mod config;
mod date;

#[derive(Clone, Debug)]
struct SuggestedPresence {
    presence: String,
    suggested_by: String,
}

#[derive(Default)]
struct Handler {
    pub settings: Settings,
    pub presences: Arc<Mutex<Vec<SuggestedPresence>>>,
}

impl Handler {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            ..Default::default()
        }
    }

    async fn handle_presence_message(&self, new_message: Message) {
        let content = new_message.content;
        let user = format!(
            "{}#{}",
            new_message.author.name, new_message.author.discriminator
        );
        let presence = SuggestedPresence {
            presence: content,
            suggested_by: user,
        };

        info!("A presence was suggested: {presence:#?}");
        self.presences.clone().lock().await.push(presence);
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, new_message: Message) {
        if new_message.channel_id == self.settings.day_channel_id {
            self.handle_presence_message(new_message).await;
        }
    }

    async fn ready(&self, ctx: Context, data_about_bot: Ready) {
        info!("Bot is ready. id: {}", data_about_bot.user.id);

        let ctx = Arc::new(ctx);

        for message in ctx
            .http
            .get_messages(self.settings.presence_channel_id, "")
            .await
            .unwrap()
        {
            self.handle_presence_message(message).await;
        }

        tokio::spawn({
            let settings = self.settings.clone();
            let ctx = ctx.clone();

            async move {
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
            }
        });

        tokio::spawn({
            let ctx = ctx.clone();
            let presences = self.presences.clone();

            async move {
                let mut rng = XorShiftRng::from_entropy();

                loop {
                    {
                        let possibilities = presences.lock().await.clone();
                        let presence = &possibilities
                            [(rng.gen::<f32>() * possibilities.len() as f32) as usize];

                        ctx.set_presence(
                            Some(Activity::playing(&presence.presence)),
                            OnlineStatus::Online,
                        )
                        .await;

                        info!("Set presence: {presence:#?}");
                    }
                    thread::sleep(Duration::from_secs(60));
                }
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
        .event_handler(Handler::new(settings))
        .await
        .expect("Failed to create client");

    if let Err(cause) = client.start().await {
        eprintln!("Starting client caused error: {:#?}", cause);
    }
}
