use log::{debug, error};
use serenity::client::Context;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::macros::group;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;
use serenity::model::guild::Guild;
use serenity::model::id::GuildId;
use songbird::input::Input;

#[group]
#[commands(play)]
pub struct Voice;

#[command]
async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    connect_to_vc(ctx, msg, &guild, &guild_id).await;

    let url = find_url(ctx, msg, args.clone()).await.unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let source = match songbird::ytdl(&url).await {
            Ok(source) => {
                if let Some(title) = &source.metadata.title {
                    msg.reply(&ctx.http, format!(r#"Playing `{title}`"#)).await.unwrap();
                }
                source
            }
            Err(_) => match search_video(&args).await {
                Ok(source) => {
                    if let Some(title) = &source.metadata.title {
                        msg.reply(&ctx.http, format!(r#"Playing `{title}`"#)).await.unwrap();
                    }
                    source
                }
                Err(why) => {
                    error!("Err starting source: {:?}", why);

                    msg.reply(&ctx.http, "Error sourcing ffmpeg").await.unwrap();
                    return Ok(());
                }
            },
        };

        debug!("source {:#?}", source);

        handler.play_only_source(source);
    } else {
        msg.reply(&ctx.http, "Not in a voice channel to play in").await.unwrap();
    }

    Ok(())
}

async fn connect_to_vc(ctx: &Context, msg: &Message, guild: &Guild, guild_id: &GuildId) {
    let channel_id = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            msg.reply(ctx, "Not in a voice channel").await.unwrap();
            return;
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    manager.join(*guild_id, connect_to).await.1.unwrap();
}

async fn find_url(ctx: &Context, msg: &Message, mut args: Args) -> Option<String> {
    match args.single::<String>() {
        Ok(mut url) => {
            let url = if !url.contains("shorts") {
                url
            } else {
                url.drain(url.rfind('/').unwrap() + 1..).collect()
            };
            Some(url)
        }
        Err(_) => {
            msg.reply(&ctx.http, "Must provide a URL to a video or audio").await.unwrap();
            None
        }
    }
}

async fn search_video(args: &Args) -> songbird::input::error::Result<Input> {
    let args = args.raw().collect::<Vec<&str>>().join(" ");
    songbird::ytdl(&format!("ytsearch1:{}", args)).await
}
