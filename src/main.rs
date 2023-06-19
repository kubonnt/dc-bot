pub mod config;
pub mod commands;
pub mod hooks;
pub mod utils;

use crate::commands::*;
use crate::hooks::*;

use chrono::offset::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use config::Config;
use serenity::{
    async_trait,
    model::gateway::Ready,
    prelude::*,
    client::{Client, Context, EventHandler},
    framework::standard::StandardFramework,
};

use serenity::http::Http;

use serenity::model::id::{ChannelId, GuildId};
use serenity::model::prelude::Activity;
use songbird::SerenityInit;
use rspotify::{
    model::{AdditionalType, Country, Market, AlbumId},
    prelude::*,
    scopes, Credentials, OAuth, ClientCredsSpotify
};

struct Handler {
    is_loop_running: AtomicBool,
}

#[async_trait]
impl EventHandler for Handler {
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        println!("Cache built successfully!");

        let ctx = Arc::new(ctx);
        if !self.is_loop_running.load(Ordering::Relaxed) {
            let ctx1 = Arc::clone(&ctx);
            tokio::spawn(async move {
                loop {
                    log_system_load(Arc::clone(&ctx1)).await;
                    tokio::time::sleep(Duration::from_secs(120)).await;
                }
            });

            let ctx2 = Arc::clone(&ctx);
            tokio::spawn(async move {
                loop {
                    set_status_to_current_time(Arc::clone(&ctx2)).await;
                    tokio::time::sleep(Duration::from_secs(120)).await;
                }
            });

            self.is_loop_running.swap(true, Ordering::Relaxed);
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

async fn log_system_load(ctx: Arc<Context>) {
    let cpu_load = sys_info::loadavg().unwrap();
    let mem_usage = sys_info::mem_info().unwrap();

    let message = ChannelId(1107344555159855115)
        .send_message(&ctx, |m| {
            m.embed(|e| {
                e.title("System Resource Load")
                    .field("CPU Load Average", format!("{:.2}", cpu_load.one * 10.0), false)
                    .field(
                        "Memory Usage",
                        format!(
                            "{:.2} MB Free out of {:.2} MB",
                            mem_usage.free as f32 / 1000.0,
                            mem_usage.total as f32 / 1000.0
                        ),
                        false,
                    )
            })
        }).await;
    if let Err(why) = message {
        eprintln!("Error sending message: {:?}", why);
    };
}

async fn set_status_to_current_time(ctx: Arc<Context>) {
    let current_time = Utc::now();
    let formatted_time = current_time.to_rfc2822();

    ctx.set_activity(Activity::playing(&formatted_time)).await;
}

#[tokio::main]
async fn main() {
    // Discord bot init
    let _ = Config::new().save();
    let config = Config::load().unwrap();

    {
        let creds = Credentials {
            id: config.spotify_client_id().to_string(),
            secret: Some(config.spotify_client_secret().to_string()),
        };

        let spotify = ClientCredsSpotify::new(creds);

        spotify.request_token().await.unwrap();

        let birdy_uri = AlbumId::from_uri("spotify:album:0sNOF9WDwhWunNAHPD3Baj").unwrap();
        let albums = spotify.album(birdy_uri).await;

        println!("Response: {albums:#?}");
    }

    let http = Http::new(&config.token());

    let (owners, bot_id) = match http.get_current_application_info().await {
      Ok(info) => {
          let mut owners = HashSet::new();
          if let Some(team) = info.team {
              owners.insert(team.owner_user_id);
          } else {
              owners.insert(info.owner.id);
          }
          match http.get_current_user().await {
              Ok(bot_id) => (owners, bot_id.id),
              Err(why) => panic!("Could not bot id: {:?}", why),
          }
      },
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    let framework = StandardFramework::new()
        .configure(|c|{ c
            .prefix(config.prefix())
            .on_mention(Some(bot_id))
            .with_whitespace(true)
            .owners(owners)
        })
        .before(before)
        .after(after)
        .normal_message(normal_message)
        .unrecognised_command(unknown_command)
        .on_dispatch_error(dispatch_error)
        .group(&GENERAL_GROUP)
        .group(&OWNER_GROUP)
        .help(&MY_HELP)
        .bucket("complicated", |b| b.delay(5)).await;

    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILD_VOICE_STATES;

    let mut client = Client::builder(&config.token(), intents)
        .event_handler(Handler {
            is_loop_running: AtomicBool::new(false),
        })
        .framework(framework)
        .register_songbird()
        .type_map_insert::<CommandCounter>(HashMap::default())
        .await
        .expect("Error creating client!");
    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
    }

    if let Err(error) = client.start().await {
        println!("Client error: {}.", error);
    }
}