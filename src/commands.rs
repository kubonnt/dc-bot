use std::sync::Arc;

use serenity::{
    model::{channel::Message},
    prelude::*,
    client::{Context},
    framework::standard::{
        macros::{command, group},
        CommandResult,
    },
};

use serenity::client::bridge::gateway::{ShardManager, ShardId};
use tokio::sync::Mutex;

struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

#[group]
#[commands(ping, create_channel, latency)]
struct General;

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    if let Err(error) = msg.channel_id.say(&ctx.http, "Pong").await {
        println!("Error sending message: {}.", error);
    }
    return Ok(());
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
            msg.reply(ctx, "There was a problem getting the shard manager.").await?;

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


