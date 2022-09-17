extern crate core;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use log::{info, LevelFilter};
use rand::prelude::SliceRandom;
use rand::SeedableRng;
use rand_xorshift::XorShiftRng;
use serde_json::Value;
use serenity::framework::StandardFramework;
use serenity::json::hashmap_to_json_map;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::hashmap_to_json_map;
use songbird::SerenityInit;

use crate::config::Settings;

mod commands;
mod config;
mod date;

#[derive(Clone, Debug, PartialEq)]
struct SuggestedPresence {
    content: String,
}

#[derive(Default)]
struct Handler {
    pub settings: Settings,
    pub presences: Arc<Mutex<HashMap<u64, SuggestedPresence>>>,
}

impl Handler {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            ..Default::default()
        }
    }

    pub async fn register_new_presence(&self, message: Message) {
        let MessageId(id) = message.id;
        let new_presence = SuggestedPresence {
            content: message.content,
        };

        info!("Registered presence: {new_presence:#?}");
        self.presences.lock().await.insert(id, new_presence);
    }

    pub async fn unregister_presence(&self, message_id: u64) {
        if let Some(removed_presence) = self.presences.lock().await.remove(&message_id) {
            info!("Deleted presence: {removed_presence:#?}");
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, new_message: Message) {
        if new_message.channel_id == self.settings.presence_channel_id {
            self.register_new_presence(new_message).await;
        }
    }

    async fn message_delete(
        &self,
        _ctx: Context,
        ChannelId(channel_id): ChannelId,
        MessageId(message_id): MessageId,
        _guild_id: Option<GuildId>,
    ) {
        if channel_id == self.settings.presence_channel_id {
            self.unregister_presence(message_id).await;
        }
    }

    async fn ready(&self, ctx: Context, data_about_bot: Ready) {
        info!("Bot is ready. id: {}", data_about_bot.user.id);

        // bad code
        for message in ctx
            .http
            .get_messages(self.settings.presence_channel_id, "")
            .await
            .unwrap()
        {
            self.register_new_presence(message).await;
        }

        let ctx = Arc::new(ctx);
        // task responsible for renaming channel to current day of the week
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
                            .edit_channel(
                                settings.day_channel_id,
                                &hashmap_to_json_map(map),
                                Some("Automatic channel rename"),
                            )
                            .await
                            .expect("failed to rename channel");

                        info!("Renamed channel. Endlich {current_week_day}!");
                    }

                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        });

        // task responsible for setting the discord presence
        tokio::spawn({
            let presences = self.presences.clone();

            async move {
                let mut rng = XorShiftRng::from_entropy();
                let mut last_presence = None;

                loop {
                    let possibilities = presences
                        .lock()
                        .await
                        .clone()
                        .into_values()
                        .filter(|presence| last_presence.as_ref() != Some(presence))
                        .collect::<Vec<_>>();

                    if let Some(presence) = possibilities.choose(&mut rng) {
                        let activity = Activity::playing(&presence.content.clone());
                        ctx.set_activity(activity.clone()).await;

                        info!("Set presence: {presence:#?}");
                        tokio::time::sleep(Duration::from_secs(30)).await;

                        last_presence = Some(presence.clone());
                    }
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
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!"))
        .group(&commands::voice::VOICE_GROUP);

    let mut client = Client::builder(&settings.token, GatewayIntents::all())
        .event_handler(Handler::new(settings))
        .framework(framework)
        .register_songbird()
        .await
        .expect("Failed to create client");

    if let Err(cause) = client.start().await {
        eprintln!("Starting client caused error: {:#?}", cause);
    }
}
