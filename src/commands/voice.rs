use crate::Error;
use log::debug;
use reqwest::Client;
use serenity::all::ChannelId;
use serenity::all::ChannelType;
use serenity::async_trait;
use serenity::client::Context;
use songbird::events::{Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent};
use songbird::input::Compose;
use songbird::input::YoutubeDl;

use crate::PoiseContext;

struct TrackErrorNotifier;

#[async_trait]
impl VoiceEventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                println!(
                    "Track {:?} encountered an error: {:?}",
                    handle.uuid(),
                    state.playing
                );
            }
        }

        None
    }
}

#[poise::command(slash_command, prefix_command)]
pub async fn stop(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let serenity_ctx = ctx.serenity_context();

    let manager = songbird::get(serenity_ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let guild_id = ctx.guild_id().expect("Unable to find guild id");
    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        // Stop the song playback
        handler.stop();

        // Send feedback message
        ctx.reply("Successfully stopped the video").await?;
    } else {
        ctx.reply("Couldn't find bot in voice channel, unable to stop the video!")
            .await?;
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn play(
    ctx: PoiseContext<'_>,
    #[description = "The url or video name to play"] video: String,
) -> Result<(), Error> {
    let serenity_ctx = ctx.serenity_context();
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => {
            ctx.reply("Unable to find channel").await?;
            return Ok(());
        }
    };

    connect_to_vc(serenity_ctx, ctx).await;

    let manager = songbird::get(serenity_ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        let mut ytdl = YoutubeDl::new(Client::new(), video.clone());

        // Search with url first, then with text query
        let mut source = match ytdl.search(None).await {
            Ok(_) => ytdl,
            Err(_) => {
                let mut ytdl = YoutubeDl::new_search(Client::new(), video.clone());
                match ytdl.search(Some(1)).await {
                    Ok(_) => ytdl,
                    Err(why) => {
                        ctx.reply(format!("Error playing video (detailed: {:#?})", why))
                            .await?;
                        return Ok(());
                    }
                }
            }
        };

        // We can only play the target video if an aux metadata is present
        match &source.aux_metadata().await {
            Ok(metadata) => {
                // It would be very unusual for a video to not have a title, lets handle it properly anyways
                if let Some(title) = &metadata.title {
                    ctx.reply(format!("Now playing video `{}`", title)).await?;
                } else {
                    ctx.reply("Playing unknown song title").await?;
                }

                handler.play_only_input(source.clone().into());
            }
            Err(_) => {
                ctx.reply("Unable to find video metadata").await?;
            }
        };
    } else {
        ctx.reply("Not in a voice channel to play in").await?;
    }

    Ok(())
}

/// Connects to the voice channel the user which sent the command is inside of
async fn connect_to_vc(ctx: &Context, poise_ctx: PoiseContext<'_>) -> Option<ChannelId> {
    let guild_id = poise_ctx.guild_id().unwrap();
    let guild = match guild_id.channels(&ctx.http).await {
        Ok(channels) => channels,
        Err(_) => {
            return None;
        }
    };
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let connected_channel = guild
        .values()
        .filter(|channel| channel.kind == ChannelType::Voice)
        .find(|channel| {
            let members = match channel.members(&ctx.cache) {
                Ok(members) => members,
                Err(_) => {
                    // Failed to get members from channel = most likely no one in the channel
                    return false;
                }
            };
            members
                .iter()
                .any(|member| member.user.id == poise_ctx.author().id)
        })?
        .id;

    match manager.join(guild_id, connected_channel).await {
        Ok(call) => {
            let mut handler = call.lock().await;
            handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
            debug!("Added event-handler for TrackErrorNotifier");
        }
        Err(_) => {}
    };
    Some(connected_channel)
}
