use std::collections::HashSet;
use std::fmt::Write;
use std::fmt::format;
use std::path;
use std::sync::Arc;

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
use serenity::model::application::command::CommandOption;
use serenity::model::id::UserId;
use serenity::model::{Permissions, Timestamp};
use serenity::model::channel::AttachmentType;
use serenity::model::channel::AttachmentType::Path;
use tokio::sync::Mutex;
use crate::hooks::CommandCounter;

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
#[commands(ping, latency, commands, about_role, about, am_i_admin, jd)]
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




