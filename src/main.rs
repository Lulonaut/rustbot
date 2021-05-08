use std::env;
use std::time::Duration;

use dotenv::dotenv;
use lazy_static::lazy_static;
use serde_json::Value;
use serenity::client::bridge::gateway::GatewayIntents;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::guild::Member;
use serenity::model::id::GuildId;
use serenity::Client;
use serenity::{async_trait, prelude::*};

use crate::PossibleErrors::{DiscordNotLinked, HypixelAPIError, MojangAPIError};

lazy_static! {
    static ref PREFIX: String = env::var("PREFIX").expect("Please add a PREFIX to the .env");
    static ref API_KEY: String =
        env::var("HYPIXEL_API_KEY").expect("Please add a HYPIXEL_API_KEY to the .env");
}

struct Handler;

#[derive(Debug, PartialEq)]
pub enum PossibleErrors {
    HypixelAPIError,
    MojangAPIError,
    DiscordNotLinked,
}

async fn get_discord(username: String) -> Result<String, PossibleErrors> {
    let client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(20))
        .build()
        .unwrap();

    let uuid_response = client
        .get(format!(
            "https://api.mojang.com/users/profiles/minecraft/{}",
            username
        ))
        .send()
        .await;

    if uuid_response.is_err() {
        return Err(MojangAPIError);
    }

    let json = serde_json::from_str(uuid_response.unwrap().text().await.unwrap().as_str());
    if json.is_err() {
        return Err(MojangAPIError);
    }
    let json: Value = json.expect("");

    let mut uuid;
    match json.get("id") {
        Some(v) => uuid = v.to_string(),
        None => return Err(MojangAPIError),
    }
    uuid = uuid.replace("\"", "");

    let response = client
        .get(format!(
            "https://api.hypixel.net/player?key={}&uuid={}",
            API_KEY.to_string(),
            uuid
        ))
        .send()
        .await;

    if response.is_err() {
        return Err(HypixelAPIError);
    }

    let json = serde_json::from_str(response.unwrap().text().await.unwrap().as_str());
    if json.is_err() {
        return Err(HypixelAPIError);
    }
    let json: Value = json.expect("");

    if let Some(player) = json.get("player") {
        if let Some(social_media) = player.get("socialMedia") {
            if let Some(links) = social_media.get("links") {
                if let Some(discord) = links.get("DISCORD") {
                    //slice quotation marks of string
                    let discord = discord;
                    let sliced = &discord.to_string()[1..discord.to_string().len() - 1];
                    return Ok(sliced.to_string());
                }
            }
        }
    }
    Err(DiscordNotLinked)
}

async fn say_something(message: String, ctx: Context, msg: Message) {
    if let Err(_) = msg.channel_id.say(&ctx.http, message).await {}
    return;
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
            say_something(
                format!("Invalid usage: `{}verify Username`", PREFIX.to_string()),
                ctx,
                msg,
            )
            .await;
            return;
        }
        //get linked username
        let mut iter = msg.content.splitn(2, " ");
        let _ = iter.next().unwrap();
        let username = iter.next().unwrap();
        if 3 > username.len() || username.len() > 16 {
            say_something(
                format!(
                    "Your Username is `{}` characters long, which is impossible (3-16 characters)",
                    username.len().to_string()
                ),
                ctx,
                msg,
            )
            .await;
            return;
        }

        let discord = get_discord(String::from(username)).await;
        if discord.is_err() {
            let error = discord.err().unwrap();
            if error == PossibleErrors::DiscordNotLinked {
                say_something("This User doesn't have any Discord linked on Hypixel. If you just changed it wait a few minutes and try again.".to_string(), ctx, msg).await;
                return;
            }

            if error == PossibleErrors::MojangAPIError {
                say_something("There was an Error while contacting the Mojang API or it returned bad data. Please try again later.".to_string(), ctx, msg).await;
                return;
            }
            if error == PossibleErrors::HypixelAPIError {
                say_something("There was an Error while contacting the Hypixel API or it returned bad data. Please try again later.".to_string(), ctx, msg).await;
                return;
            }
            say_something("There was an unhandled Error :(".to_string(), ctx, msg).await;
            return;
        }
        println!("{}", discord.ok().unwrap())
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
