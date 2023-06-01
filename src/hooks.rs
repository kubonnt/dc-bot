use std::collections::HashMap;
use serenity::framework::standard::{CommandError, DispatchError};
use serenity::framework::standard::macros::hook;
use serenity::model::channel::Message;
use serenity::prelude::*;

pub(crate) struct CommandCounter;

impl TypeMapKey for CommandCounter {
    type Value = HashMap<String, u64>;
}

#[hook]
pub(crate) async fn before(ctx: &Context, msg: &Message, command_name: &str) -> bool {
    println!("Got command '{}' by user '{}'", command_name, msg.author.name);

    let mut data = ctx.data.write().await;
    let counter = data.get_mut::<CommandCounter>()
        .expect("Expected CommandCounter in TypeMap");
    let entry = counter.entry(command_name.to_string()).or_insert(0);
    *entry += 1;

    true
}

#[hook]
pub(crate) async fn after(_ctx: &Context, _msg: &Message, command_name: &str, command_result: Result<(), CommandError>) {
    match command_result {
        Ok(()) => println!("Processed command '{}'", command_name),
        Err(why) => println!("Command '{}' returned error {:?}", command_name, why),
    }
}

#[hook]
pub(crate) async fn unknown_command(ctx: &Context, msg: &Message, unknown_command_name: &str) {
    msg.reply(ctx,"Could not find this command.").await.expect("Error");
    println!("Could not find command named '{}'", unknown_command_name);
}

#[hook]
pub(crate) async fn normal_message(_ctx: &Context, msg: &Message) {
    println!("Message is not a command '{}'", msg.content);
}

#[hook]
pub(crate) async fn delay_action(ctx: &Context, msg: &Message) {
    let _ = msg.react(ctx, '⏱').await;
}

#[hook]
pub(crate) async fn dispatch_error(ctx: &Context, msg: &Message,error: DispatchError, _command_name: &str) {
    if let DispatchError::Ratelimited(info) = error {
        if info.is_first_try {
            let _ = msg
                .channel_id
                .say(&ctx.http, &format!("Try this again in {} seconds.", info.as_secs()))
                .await;
        }
    }
}