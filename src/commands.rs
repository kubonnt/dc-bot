use std::collections::HashSet;
use std::fmt::Write;
use std::path;
use std::sync::Arc;
use regex::Regex;

use serenity::{
    model::{channel::Message},
    prelude::*,
    client::{Context},
    framework::standard::{
        macros::{command, group, help, check},
        CommandResult,
    },
};

use serenity::client::bridge::gateway::{ShardManager, ShardId};
use serenity::framework::standard::{Args, CommandGroup, CommandOptions, help_commands, HelpOptions, Reason};
use serenity::http::CacheHttp;
use serenity::model::id::UserId;
use serenity::model::{Permissions, Timestamp};
use serenity::model::channel::AttachmentType::Path;
use tokio::sync::Mutex;

use crate::hooks::CommandCounter;
use crate::utils::to_time;

use songbird::input::Restartable;
use tokio::process::Command;
use tracing::{error, info};

//pub mod utils;

pub(crate) struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

#[group]
#[owners_only]
#[summary = "Commands for server owners."]
#[commands(create_channel)]
struct Owner;

#[group]
#[commands(ping, latency, commands, about_role, about, am_i_admin, jd, przepros, join, play)]
#[summary = "Commands for server members."]
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

#[command]
#[allowed_roles("Sugar Daddy", "Pachołek")]
async fn about_role(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let potential_role_name = args.rest();

    if let Some(guild) = msg.guild(&ctx.cache) {
        if let Some(role) = guild.role_by_name(potential_role_name) {
            if let Err(why) = msg.channel_id
                .say(&ctx.http, &format!("Role-ID {}", role.id)).await {
                println!("Error sending  message {:?}", why);
            }

            return Ok(());
        }
    }

    msg.channel_id
        .say(&ctx.http, format!("Could not find role named: {:?}", potential_role_name))
        .await?;

    Ok(())
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

#[command]
async fn jd(ctx: &Context, msg: &Message) -> CommandResult {
    if let Err(error) = msg.channel_id.send_message(&ctx.http, |m| {
        m.content("JD!").add_file(Path(path::Path::new("media/dis.png")))
    }).await {
        println!("Error sending message: {:?}.", error);
    }

    Ok(())
}

#[command]
async fn create_channel(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx,"Not yet implemented.").await?;
    !unimplemented!()
}

#[command]
async fn latency(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let shard_manager = match data.get::<ShardManagerContainer>() {
        Some(v) => v,
        None => {
            msg.reply(ctx, "There was a problem getting the shard manager. (Nie przejmuj się tym.)").await?;

            return Ok(());
        },
    };

    let manger = shard_manager.lock().await;
    let runners = manger.runners.lock().await;

    let runner = match runners.get(&ShardId(ctx.shard_id)) {
        Some(runner) => runner,
        None => {
            msg.reply(ctx, "No shard found.").await?;

            return Ok(());
        },
    };

    msg.reply(ctx, &format!("The shard latency is {:?}", runner.latency)).await?;

    Ok(())
}

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
            msg.channel_id.send_message(&ctx.http, |m| {
                m.embed(|e| {
                    e.colour(0xf38ba8)
                        .title(":warning: Use command like this: play <url>")
                        .timestamp(Timestamp::now())
                })
            }).await?;
            return Ok(());
        }
    };
    let search = args.clone();

    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at init.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        if !url.starts_with("http") {
            let source = match songbird::input::ytdl_search(search.message()).await {
                Ok(source) => source,
                Err(why) => {
                  println!("Error starting source: {:?}", why);

                    msg.channel_id.send_message(&ctx.http, |m| {
                        m.embed(|e| {
                            e.colour(0xf38ba8)
                                .title(":warning: Error adding song to playlist.")
                                .description("Probably one of the songs in the playlist isn't available.")
                                .timestamp(Timestamp::now())
                        })
                    }).await?;
                    return Ok(());
                },
            };

            let song = handler.enqueue_source(source.into());
            let mut i = 0;
            for queued_song in handler.queue().current_queue() {
                i += queued_song.metadata().duration.unwrap().as_secs();
            }

            let playtime = to_time(i);
            let metadata = song.metadata();

            msg.channel_id.send_message(&ctx.http, |m| {
                m.embed(|e| {
                    e.colour(0xa6e3a1)
                        .title(":notes: Found song!")
                        .description(format!(
                            "{} - {}",
                            metadata.title.clone().unwrap(),
                            metadata.artist.clone().unwrap()
                        ))
                        .fields(vec![
                            ("Songs queued", format!("{}", handler.queue().len()), true),
                            ("Total playtime", playtime, true)
                        ])
                        .timestamp(Timestamp::now())
                })
            }).await?;

        } else if url.contains("playlist") {
            let get_raw_list = Command::new("yt-dlp")
                .args(&["-j", "--flat-playlist", &url])
                .output()
                .await;

            let raw_list = match get_raw_list {
                Ok(list) => String::from_utf8(list.stdout).unwrap(),
                Err(_) => String::from("Error!"),
            };

            let re = Regex::new(r#""url": "(https://www.youtube.com/watch\?v=[A-Za-z0-9]{11})""#).unwrap();
            let urls: Vec<String> = re.captures_iter(&raw_list)
                .map(|cap| cap[1].to_string())
                .collect();

            for url in urls {
                info!("Queueing --> {}", url);
                let source = match Restartable::ytdl(url, true).await {
                    Ok(source) => source,
                    Err(why) => {
                        error!("Error starting: {:?}", why);

                        msg.channel_id.send_message(&ctx.http, |m| {
                            m.embed(|e| {
                                e.colour(0xf38ba8)
                                    .title(":warning: Error adding song to the playlist.")
                                    .description("This could mean that the song is unavailable.")
                                    .timestamp(Timestamp::now())
                            })
                        }).await?;
                        return Ok(());
                    }
                };

                let _song = handler.enqueue_source(source.into());
                let mut i = 0;
                for queued_song in handler.queue().current_queue() {
                    i += queued_song.metadata().duration.unwrap().as_secs();
                }

                let playtime = to_time(i);

                msg.channel_id.send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        e.colour(0xa6e3a1)
                            .title(":notes: Added playlist!")
                            .fields(vec![
                                ("Songs queued", format!("{}", handler.queue().len()), true),
                                ("Total playtime", playtime, true)
                            ])
                            .timestamp(Timestamp::now())
                    })
                }).await?;
            }
        } else {
            println!("I'm here!, url: {}", url);
            let source = match Restartable::ytdl(url, true).await {
                Ok(source) => source,
                Err(why) => {
                    println!("Error starting: {:?}", why);

                    msg.channel_id.send_message(&ctx.http, |m| {
                        m.embed(|e| {
                            e.colour(0xf38ba8)
                                .title(":warning: Error adding song to the playlist.")
                                .description("This could mean that the song is unavailable.")
                                .timestamp(Timestamp::now())
                        })
                    }).await?;
                    return Ok(());
                }
            };

            let song = handler.enqueue_source(source.into());
            let mut i = 0;
            for queued_song in handler.queue().current_queue() {
                i += queued_song.metadata().duration.unwrap().as_secs();
            }

            let playtime = to_time(i);
            let metadata = song.metadata();

            msg.channel_id.send_message(&ctx.http, |m| {
                m.embed(|e| {
                    e.colour(0xa6e3a1)
                        .title(":notes: Found song!")
                        .description(format!(
                            "{} - {}",
                            metadata.title.clone().unwrap(),
                            metadata.artist.clone().unwrap()
                        ))
                        .fields(vec![
                            ("Songs queued", format!("{}", handler.queue().len()), true),
                            ("Total playtime", playtime, true)
                        ])
                        .timestamp(Timestamp::now())
                })
            }).await?;
        }
    } else {
        msg.channel_id.send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.colour(0xf38ba8)
                    .title(":warning: Not in the voice channel.")
                    .timestamp(Timestamp::now())
            })
        }).await?;
    }

    Ok(())
}

#[command]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    println!("{:?}\n{:?}", msg, _args);

    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    //let debug_channel_id: i64 = 1107375027952828507;
    let channel_id = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            msg.channel_id.send_message(&ctx.http, |m| {
                m.embed(|e| {
                    e.colour(0xf38ba8)
                        .title(":warning: Join a voice channel first!")
                        .timestamp(Timestamp::now())
                })
            }).await?;

            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird voice client init.")
        .clone();

    let(_, success) = manager.join(guild_id, connect_to).await;

    if let Ok(_channel) = success {
        msg.channel_id.send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.colour(0xa6e3a1)
                    .title(format!("Joined channel --> {}", connect_to.mention()))
                    .timestamp(Timestamp::now())
            })
        }).await?;
    } else {
        msg.channel_id.send_message(&ctx.http, |m| {
                m.embed(|e| {
                    e.colour(0xf38ba8)
                        .title(":warning: Error joining channel.")
                        .description("Please ensure I have the correct permissions.")
                        .timestamp(Timestamp::now())
                })
            }).await?;
    }

    Ok(())
}

