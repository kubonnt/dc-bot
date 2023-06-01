pub mod config;
pub mod commands;
pub mod hooks;

use crate::commands::GENERAL_GROUP;
use crate::hooks::*;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use config::Config;
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
    client::{Client, Context, EventHandler},
    framework::standard::{
        macros::{help, hook},
        CommandResult,
        StandardFramework,
    },
};

use serenity::http::Http;
use serenity::framework::standard::{Args, CommandGroup, DispatchError, help_commands, HelpOptions};
use serenity::model::id::UserId;

use rspotify::{
    model::{AdditionalType, Country, Market},
    prelude::*,
    scopes, AuthCodeSpotify, Credentials, OAuth,
};

const SIMPLE_RESPONSE: &str = "Przepraszam, mój panie.";
const SIMPLE_COMMAND: &str = "!przepros";

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == SIMPLE_COMMAND {
            println!("Inside!");
            if let Err(error) = msg.channel_id.say(&ctx.http, SIMPLE_RESPONSE).await {
                println!("Error sending the message: {}", error);
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}


#[help]
#[individual_command_tip = "Siemka! Po wiecej info o komendach, po prostu podaj komende po '!'"]
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

#[tokio::main]
async fn main() {
    let _ = Config::new().save();
    let config = Config::load().unwrap();

    let http = Http::new(&config.token());
    let bot_id = match http.get_current_user().await {
        Ok(bot_id) => bot_id.id,
        Err(why) => panic!("Could not access the bot id: {:?}", why),
    };

    let framework = StandardFramework::new()
        .configure(|c|{ c
            .prefix(config.prefix())
            .on_mention(Some(bot_id))
            .with_whitespace(true)
        })
        .before(before)
        .after(after)
        .normal_message(normal_message)
        .unrecognised_command(unknown_command)
        .group(&GENERAL_GROUP)
        .help(&MY_HELP)
        .on_dispatch_error(dispatch_error);

    let intents = GatewayIntents::default()
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::DIRECT_MESSAGES;

    let mut client = Client::builder(&config.token(), intents)
        .event_handler(Handler)
        .framework(framework)
        .type_map_insert::<CommandCounter>(HashMap::default())
        .await
        .expect("Error creating client!");
    {
        let mut data = client.data.write().await;
        data.insert::<commands::ShardManagerContainer>(Arc::clone(&client.shard_manager));
    }

    if let Err(error) = client.start().await {
        println!("Client error: {}.", error);
    }
}