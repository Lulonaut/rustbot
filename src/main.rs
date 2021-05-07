use std::env;

use dotenv::dotenv;
use lazy_static::lazy_static;
use reqwest::{Response, Url};
use serenity::client::bridge::gateway::GatewayIntents;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::guild::Member;
use serenity::model::id::GuildId;
use serenity::Client;
use serenity::{async_trait, prelude::*};

lazy_static! {
    static ref PREFIX: String = env::var("PREFIX").expect("Please add a PREFIX to the .env");
    static ref API_KEY: String =
        env::var("HYPIXEL_API_KEY").expect("Please add a HYPIXEL_API_KEY to the .env");
}

struct Handler;

async fn get_discord(username: String) -> Option<String> {
    let uuid_response = reqwest::get(format!(
        "https://api.mojang.com/users/profiles/minecraft/{}",
        username
    ))
    .await
    .ok()?
    .text()
    .await
    .ok()?;
    let json: serde_json::Value =
        serde_json::from_str(&*uuid_response.to_string()).expect("API returned bad JSON");

    let mut uuid;
    match json.get("id") {
        Some(v) => uuid = v.to_string(),
        None => return None,
    }
    uuid = uuid.replace("\"", "");
    let response = reqwest::get(format!(
        "https://api.hypixel.net/player?key={}&uuid={}",
        API_KEY.to_string(),
        uuid
    ))
    .await
    .ok()?
    .text()
    .await
    .ok()?;

    let json: serde_json::Value =
        serde_json::from_str(&*response.to_string()).expect("API returned bad JSON");

    if let Some(player) = json.get("player") {
        if let Some(social_media) = player.get("socialMedia") {
            if let Some(links) = social_media.get("links") {
                if let Some(discord) = links.get("DISCORD") {
                    return Some(discord.to_string());
                } else {
                    println!("discord")
                };
            } else {
                println!("links")
            };
        } else {
            println!("SOCIAL MEDIA")
        };
    } else {
        println!("{}", json.get("success").expect("aa"))
    };

    None
}

#[async_trait]
impl EventHandler for Handler {
    async fn guild_member_addition(&self, ctx: Context, guild_id: GuildId, mut new_member: Member) {
        let guild = guild_id.to_guild_cached(&ctx).await.expect("guild err");
        let role_id = guild.role_by_name("Member").expect("role err");
        if let Err(why) = new_member.add_role(&ctx, role_id).await {
            println!("Error while adding role: {}", why)
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        //check if its the actual command and not a bot
        let command: String = format!("{}verify", PREFIX.to_string());
        if msg.author.bot || !msg.content.starts_with(&command) {
            return;
        }

        //check for correct usage
        let args = msg.content.split(" ");
        if args.count() != 2 {
            if let Err(_) = msg
                .channel_id
                .say(
                    &ctx.http,
                    format!("Invalid usage. `{}verify Username`", PREFIX.to_string()),
                )
                .await
            {}
            return;
        }
        let mut iter = msg.content.splitn(2, " ");
        let _ = iter.next().unwrap();
        let username = iter.next().unwrap();
        let discord = get_discord(String::from(username)).await;
        println!("{:?}", discord.unwrap())
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    dotenv().expect("Please add a .env file.");
    let token = env::var("DISCORD_TOKEN").expect("Please add a DISCORD_TOKEN to the .env");

    let mut client = Client::builder(token)
        .intents(
            GatewayIntents::GUILD_MEMBERS | GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILDS,
        )
        .event_handler(Handler)
        .await
        .expect("Error while creating Bot client");

    if let Err(why) = client.start().await {
        println!("Error {:?}", why);
    }
}
