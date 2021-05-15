use std::env;

use dotenv::dotenv;
use lazy_static::lazy_static;
use serenity::client::bridge::gateway::GatewayIntents;
use serenity::model::{channel::Message, gateway::Ready, guild::Member, id::GuildId};
use serenity::Client;
use serenity::{async_trait, prelude::*};
use tokio::runtime::Runtime;

mod commands;
mod features;

lazy_static! {
    static ref TOKEN: String =
        env::var("DISCORD_TOKEN").expect("Please add a DISCORD_TOKEN to the .env");
    static ref PREFIX: String = env::var("PREFIX").expect("Please add a PREFIX to the .env");
    static ref API_KEY: String =
        env::var("HYPIXEL_API_KEY").expect("Please add a HYPIXEL_API_KEY to the .env");
    static ref VERIFIED_ROLE: String = env::var("VERIFIED_ROLE")
        .expect("Please add a VERIFIED_ROLE to the .env")
        .replace("_", " ");
    static ref COMMAND: String = format!("{}verify", PREFIX.to_string());
}

async fn say_something(message: String, ctx: Context, msg: Message) {
    if let Err(_) = msg.channel_id.say(&ctx.http, message).await {}
    return;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn guild_member_addition(&self, ctx: Context, guild_id: GuildId, mut new_member: Member) {
        if let Some(guild) = guild_id.to_guild_cached(&ctx).await {
            if let Some(role_id) = guild.role_by_name("Member") {
                if let Err(_) = new_member.add_role(&ctx, role_id).await {}
            }
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }
        //handle message addition async
        let handle = features::message_counting::handle_messages(msg.member(&ctx).await.unwrap());
        tokio::spawn(async { handle.await });
        //execute verify command
        commands::verify_command::execute_command(
            ctx,
            msg,
            COMMAND.to_string(),
            PREFIX.to_string(),
            VERIFIED_ROLE.to_string(),
            API_KEY.to_string(),
        )
        .await;
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("Connected as {}", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    //tokio runtime
    let rt = Runtime::new().unwrap();
    let _ = rt.enter();

    dotenv().expect("please add a .env");
    let mut client = Client::builder(TOKEN.to_string())
        .intents(
            GatewayIntents::GUILD_MEMBERS | GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILDS,
        )
        .event_handler(Handler)
        .await
        .expect("Error while building Bot client");

    if let Err(why) = client.start().await {
        println!("Error while starting {:?}", why);
    }
}
