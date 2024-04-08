use std::collections::HashSet;
use std::fmt::Write;

use reqwest::Client;
use serenity::{async_trait, client::Context, framework::standard::{
    CommandResult,
    macros::{check, command, group, help},
}, model::channel::Message, prelude::*};
use serenity::all::Builder;
use serenity::framework::standard::{Args, CommandGroup, CommandOptions, help_commands, HelpOptions, Reason};
use serenity::http::CacheHttp;
use serenity::model::gateway::Ready;
use serenity::model::id::UserId;
use serenity::model::Permissions;
use songbird::{EventContext, TrackEvent};
use songbird::events::{Event, EventHandler as VoiceEventHandler};
use songbird::input::YoutubeDl;

use crate::hooks::CommandCounter;

pub(crate) struct HttpKey;

impl TypeMapKey for HttpKey {
    type Value = Client;
}

pub(crate) struct Handler;
#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

pub(crate) struct TrackErrorNotifier;
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

#[group]
#[owners_only]
#[summary = "Commands for server owners."]
#[commands(create_channel)]
struct Owner;

// #[group]
// #[commands(ping, latency, commands, about_role, about, am_i_admin, jd, przepros, join, play, leave)]
// struct General;

#[group]
#[summary = "Commands for all users."]
#[commands(ping, join, leave, play, stop, queue, reset_queue, about, am_i_admin, przepros)]
struct General;

#[help]
#[individual_command_tip = "Hej! Po wiecej info o komendach, podaj komende po wykrzykniku. "]
#[command_not_found_text = "Could not find: `{}`."]
async fn my_help(
    ctx: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(ctx, msg, args, help_options, groups, owners).await;
    Ok(())
}

#[command]
#[bucket = "complicated"]
async fn commands(ctx: &Context, msg: &Message) -> CommandResult {
    let mut contents = "Commands used:\n".to_string();

    let data = ctx.data.read().await;
    let counter = data.get::<CommandCounter>()
        .expect("Expected CommandCounter in TypeMap.");

    for (key, value) in counter {
        writeln!(contents, "- {name}: {amount}", name = key, amount = value)?;
    }

    msg.channel_id.say(&ctx.http, &contents).await?;

    Ok(())
}

#[check]
#[name= "Owner"]
async fn owner_check(
    _: &Context,
    msg: &Message,
    _: &mut Args,
    _: &CommandOptions) -> Result<(), Reason> {
    if msg.author.id != 303640900838490114 {
        return Err(Reason::User("Lacked admin permission.".to_string()));
    }

    Ok(())
}

async fn get_http_client(ctx: &Context) -> Client {
    let data = ctx.data.read().await;
    data.get::<HttpKey>()
        .cloned()
        .expect("Guaranteed to exist in typemap")
}

#[command]
async fn about(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Alfred. Poproś o co chesz, paniczu.").await?;

    Ok(())
}

#[command]
async fn am_i_admin(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    if let Some(member) = &msg.member {
        for role in &member.roles {
            if role
                .to_role_cached(&ctx.cache)
                .map_or(false, |r| r.has_permission(Permissions::ADMINISTRATOR)) {
                msg.channel_id.say(&ctx.http, "Yes, you are.").await?;

                return Ok(());
            }
        }
    }

    msg.channel_id.say(&ctx.http, "No, you are not..").await?;

    Ok(())
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    if let Err(error) = msg.channel_id.say(&ctx.http, "Pong").await {
        println!("Error sending message: {}.", error);
    }
    return Ok(());
}

// #[command]
// async fn jd(ctx: &Context, msg: &Message) -> CommandResult {
//     if let Err(error) = msg.channel_id.send_message(&ctx.http, |m: MessageBuilder| {
//         m.("JD!").add_file(Path::new("media/dis.png"))
//     }).await {
//         println!("Error sending message: {:?}.", error);
//     }
//
//     Ok(())
// }

#[command]
async fn create_channel(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx,"Not yet implemented.").await?;
    !unimplemented!()
}

// #[command]
// async fn latency(ctx: &Context, msg: &Message) -> CommandResult {
//     let data = ctx.data.read().await;
//     let shard_manager = match data.get::<ShardManagerContainer>() {
//         Some(v) => v,
//         None => {
//             msg.reply(ctx, "There was a problem getting the shard manager. (Nie przejmuj się tym.)").await?;
//
//             return Ok(());
//         },
//     };
//
//     let manger = shard_manager.lock().await;
//     let runners = manger.runners.lock().await;
//
//     let runner = match runners.get(&ShardId(ctx.shard_id)) {
//         Some(runner) => runner,
//         None => {
//             msg.reply(ctx, "No shard found.").await?;
//
//             return Ok(());
//         },
//     };
//
//     msg.reply(ctx, &format!("The shard latency is {:?}", runner.latency)).await?;
//
//     Ok(())
// }

#[command]
async fn przepros(ctx:  &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Przepraszam, mój panie.").await?;

    Ok(())
}

#[command]
async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let url = match args.single::<String>() {
        Ok(url) => url,
        Err(_) => {
            msg.channel_id.say(&ctx.http, ":warning: Use command like this: play <url>");
            return Ok(());
        }
    };

    let do_search = !url.starts_with("http");

    let guild_id = msg.guild_id.unwrap();

    let http_client = {
        let data = ctx.data.read().await;
        data.get::<HttpKey>()
            .cloned()
            .expect("Guaranteed to exist in the typemap.")
    };

    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at init.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let mut source = if do_search {
            YoutubeDl::new_search(http_client, url)
        } else {
            YoutubeDl::new(http_client, url)
        };

        handler.stop();
        handler.queue().stop();
        handler.enqueue_input(source.clone().into()).await;
        //handler.play_input(source.clone().into());


        msg.channel_id.say(&ctx.http,
        format!("Playing the song, position in the queue: position {}", handler.queue().len())
        ).await;

        handler.queue().resume().expect("TODO: panic message");
    } else {
        msg.channel_id.say(&ctx.http, ":warning: Error sourcing ffmpeg.").await;
    }

    Ok(())
}

#[command]
async fn stop(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        handler.queue().stop();
        handler.stop();

        msg.channel_id.say(&ctx.http, "Playback stopped, queue cleared.").await;
    } else {
        msg.channel_id.say(&ctx.http, "Not in voice channel.").await;
    }

    Ok(())
}

#[command]
async fn queue(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let url = match args.single::<String>() {
        Ok(url) => url,
        Err(_) => {
            msg.channel_id.say(&ctx.http, "Must provide url for audio or video.")
                .await;

            return Ok(());
        },
    };

    if !url.starts_with("http") {
        msg.channel_id.say(&ctx.http, "Must provide a valid URL.").await;

        return Ok(());
    }

    let guild_id = msg.guild_id.unwrap();

    let http_client = get_http_client(ctx).await;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let src = YoutubeDl::new(http_client, url);
        handler.enqueue_input(src.into()).await;

        msg.channel_id.say(&ctx.http,
            format!("Added song to the queue: position {}", handler.queue().len())
        ).await;
    } else {
        msg.channel_id.say(&ctx.http, "Not in a voice channel to play in").await;
    }

    Ok(())
}

#[command]
async fn reset_queue(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        queue.stop();

        msg.channel_id.say(&ctx.http, "Queue cleared.").await;
    } else {
        msg.channel_id.say(&ctx.http, "Not in voice channel.").await;
    }

    Ok(())
}

#[command]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    // println!("{:?}\n{:?}", msg, _args);

    let (guild_id, channel_id) = {
        let guild = msg.guild(&ctx.cache).unwrap();
        let channel_id = guild
            .voice_states
            .get(&msg.author.id)
            .and_then(|voice_state| voice_state.channel_id);

        (guild.id, channel_id)
    };

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            msg.channel_id.say(&ctx.http, ":warning: Join a voice channel first!");

            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird voice client init.")
        .clone();

    if let Ok(handler_lock) = manager.join(guild_id, connect_to).await {
        let mut handler = handler_lock.lock().await;
        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
    } else {
        msg.channel_id.say(&ctx.http, ":warning: Error joining channel. \
        Please ensure I have the correct permissions.").await;
    }

    Ok(())
}

#[command]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let manager = songbird::get(ctx).await
        .expect("Songbird voice client placed in at init.");
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(error) = manager.remove(guild_id).await {
            msg.channel_id.say(&ctx.http, ":warning: Error leaving channel {:?}.").await;
        }
    } else {
        msg.channel_id.say(&ctx.http, ":warning: Not in the voice channel.").await;
    }

    Ok(())
}

// #[command]
// async fn use_spotify(ctx: &Context, msg: &Message) -> CommandResult {
//     !unimplemented()
// }

