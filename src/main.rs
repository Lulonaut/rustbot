use std::env;
use std::time::Duration;

use dotenv::dotenv;
use lazy_static::lazy_static;
use serde_json::Value;
use serenity::client::bridge::gateway::GatewayIntents;
use serenity::model::guild::Role;
use serenity::model::{channel::Message, gateway::Ready, guild::Member, id::GuildId};
use serenity::Client;
use serenity::{async_trait, prelude::*};

use crate::PossibleErrors::{DiscordNotLinked, HypixelAPIError, InvalidUsername, MojangAPIError};

lazy_static! {
    static ref TOKEN: String =
        env::var("DISCORD_TOKEN").expect("Please add a DISCORD_TOKEN to the .env");
    static ref PREFIX: String = env::var("PREFIX").expect("Please add a PREFIX to the .env");
    static ref API_KEY: String =
        env::var("HYPIXEL_API_KEY").expect("Please add a HYPIXEL_API_KEY to the .env");
    static ref VERIFIED_ROLE: String =
        env::var("VERIFIED_ROLE").expect("Please add a VERIFIED_ROLE to the .env");
    static ref COMMAND: String = format!("{}verify", PREFIX.to_string());
}

#[derive(PartialEq)]
enum PossibleErrors {
    HypixelAPIError,
    MojangAPIError,
    DiscordNotLinked,
    InvalidUsername,
}

#[derive(PartialEq)]
enum HypixelRanks {
    Default,
    VIP,
    VIPPLUS,
    MVP,
    MVPPLUS,
    MVPPLUSPLUS,
}

struct ApiInfo {
    discord: String,
    rank: HypixelRanks,
    username: String,
}

async fn get_rank_role(rank: HypixelRanks, ctx: &Context, msg: &Message) -> Option<Role> {
    let rank_string;
    match rank {
        HypixelRanks::MVPPLUSPLUS => rank_string = "MVP++",
        HypixelRanks::MVPPLUS => rank_string = "MVP+",
        HypixelRanks::MVP => rank_string = "MVP",
        HypixelRanks::VIPPLUS => rank_string = "VIP+",
        HypixelRanks::VIP => rank_string = "VIP",
        HypixelRanks::Default => return None,
    }
    if let Some(guild_id) = msg.guild_id {
        if let Some(guild) = guild_id.to_guild_cached(&ctx).await {
            if let Some(role_id) = guild.role_by_name(rank_string) {
                return Some(role_id.clone());
            }
        }
    }
    return None;
}

fn get_rank(api_response: &Value) -> HypixelRanks {
    //based on my API Wrapper in Java: https://github.com/Lulonaut/HypixelAPIWrapper/blob/d43c73c00f2bc111cf407c6a325cb686a3ec899a/src/main/java/de/lulonaut/wrapper/utils/getStuff.java#L19
    let player = api_response.get("player").unwrap();

    //check for owner and other weird ranks (eg: Technoblade Pig rank)
    if let Some(_) = player.get("prefix") {
        return HypixelRanks::Default;
    }
    //check for staff
    if let Some(rank) = player.get("rank") {
        if rank == "HELPER" || rank == "MODERATOR" || rank == "ADMIN" || rank == "YOUTUBER" {
            return HypixelRanks::Default;
        }
    }
    //check for MVP++
    if let Some(mpr) = player.get("monthlyPackageRank") {
        if mpr.as_str().unwrap() == "SUPERSTAR" {
            return HypixelRanks::MVPPLUSPLUS;
        }
    }
    if let Some(rank) = player.get("newPackageRank") {
        let rank = rank.as_str().unwrap();
        match rank {
            "MVP_PLUS" => return HypixelRanks::MVPPLUS,
            "MVP" => return HypixelRanks::MVP,
            "VIP_PLUS" => return HypixelRanks::VIPPLUS,
            "VIP" => return HypixelRanks::VIP,
            _ => {}
        }
    }

    return HypixelRanks::Default;
}

fn get_username(api_response: &Value) -> String {
    return api_response
        .get("player")
        .unwrap()
        .get("displayname")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
}

async fn get_info(username: String) -> Result<ApiInfo, PossibleErrors> {
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

    //check for invalid username
    if let Some(_) = json.get("error") {
        return Err(InvalidUsername);
    }

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
    if let Some(_) = json.get("player") {
    } else {
        return Err(HypixelAPIError);
    }
    let rank = get_rank(&json);
    let username = get_username(&json);

    if let Some(player) = json.get("player") {
        if let Some(social_media) = player.get("socialMedia") {
            if let Some(links) = social_media.get("links") {
                if let Some(discord) = links.get("DISCORD") {
                    //slice quotation marks off string
                    let sliced = &discord.to_string()[1..discord.to_string().len() - 1];
                    return Ok(ApiInfo {
                        discord: sliced.to_string(),
                        rank,
                        username,
                    });
                }
            }
        }
    } else {
        return Err(HypixelAPIError);
    }
    Err(DiscordNotLinked)
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
                if let Err(why) = new_member.add_role(&ctx, role_id).await {
                    println!("Error while adding role: {}", why)
                }
            }
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        //check if its the actual command and not a bot
        if msg.author.bot || !msg.content.starts_with(&COMMAND.to_string()) {
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
        //get username
        let mut iter = msg.content.splitn(2, " ");
        let _ = iter.next().unwrap();
        let username = iter.next().unwrap();
        if 3 > username.len() || username.len() > 16 {
            say_something(
                format!(
                    "Your Username is `{}` characters long, which is impossible (3-16 characters). Please provide a valid Username and try again.",
                    username.len().to_string()
                ),
                ctx,
                msg,
            )
                .await;
            return;
        }
        //get discord linked to username
        let discord = get_info(String::from(username)).await;
        if discord.is_err() {
            let error = discord.err().unwrap();
            if error == PossibleErrors::DiscordNotLinked {
                say_something("This User doesn't have any Discord linked on Hypixel. If you just changed it wait a few minutes and try again.".to_string(), ctx, msg).await;
                return;
            }

            if error == PossibleErrors::MojangAPIError {
                say_something("There was an Error while contacting the Mojang API or it returned bad data (maybe an invalid Username). Please try again later.".to_string(), ctx, msg).await;
                return;
            }
            if error == PossibleErrors::HypixelAPIError {
                say_something("There was an Error while contacting the Hypixel API or it returned bad data. Please try again later.".to_string(), ctx, msg).await;
                return;
            }
            if error == PossibleErrors::InvalidUsername {
                say_something(
                    "Invalid Username (no UUID from Mojang API). Please try again.".to_string(),
                    ctx,
                    msg,
                )
                .await;
                return;
            }

            say_something("There was an unhandled Error :(".to_string(), ctx, msg).await;
            return;
        }

        let discord = discord.ok().unwrap();
        let linked_discord = discord.discord;
        let rank = discord.rank;
        let username = discord.username;

        let user_discord: String =
            msg.author.name.to_string() + "#" + &*msg.author.discriminator.to_string();

        if !(linked_discord == user_discord) {
            say_something(format!("The linked Username `{}` doesn't match your Discord Username: `{}`. If you just changed this wait a bit and try again.", linked_discord, user_discord), ctx, msg).await;
            return;
        }
        //assign Verified role
        let member = msg.member(&ctx).await;
        if member.is_err() {
            say_something("There was an Error while fetching your profile from the Discord API and therefore the bot can't assign you the roles. Please try again later".to_string(), ctx, msg).await;
            return;
        }
        let mut member = member.unwrap();

        if let Some(guild_id) = msg.guild_id {
            if let Some(guild) = guild_id.to_guild_cached(&ctx).await {
                if let Some(role_id) = guild.role_by_name(VERIFIED_ROLE.as_str()) {
                    if let Err(_) = member.add_role(&ctx, role_id).await {
                    } else {
                        //add new rank role and remove existing ones
                        let current_roles = &member.roles;
                        for i in current_roles {
                            if let Some(role) = i.to_role_cached(&ctx).await {
                                //remove existing rank roles
                                if role.name == "VIP".to_string()
                                    || role.name == "VIP+".to_string()
                                    || role.name == "MVP".to_string()
                                    || role.name == "MVP+".to_string()
                                    || role.name == "MVP++".to_string()
                                {
                                    //is it so hard to reference a variable a few times without borrowing and copying and whatever?
                                    let mut member2 = msg.member(&ctx).await.unwrap();
                                    if let Err(_) = member2.remove_role(&ctx, i).await {
                                        say_something("Some kind of Error occurred while trying to give you the role for your Rank. This probably has to do something with permissions: Make sure the bot is over you in the Role hierarchy otherwise it can't assign you the roles.".to_string(), ctx, msg).await;
                                        return;
                                    }
                                }
                            }
                        }
                        //add current rank role
                        if let Some(role) = get_rank_role(rank, &ctx, &msg).await {
                            if let Err(_) = member.add_role(&ctx, role.id).await {
                                say_something("Some kind of Error occurred while trying to give you the role for your Rank. This probably has to do something with permissions: Make sure the bot is over you in the Role hierarchy otherwise it can't assign you the roles. Also make sure the roles exist.".to_string(), ctx, msg).await;
                                return;
                            }
                        }
                        if let Err(_) = member.edit(&ctx, |m| m.nickname(username)).await {
                            say_something("The bot was unable to change your nickname. This probably has to do something with permissions: Make sure the bot is over you in the Role hierarchy otherwise it can't assign change your nickname.".to_string(), ctx, msg).await;
                            return;
                        }

                        say_something(
                            "You now have all the roles and your Nickname was changed to your Minecraft Username.".to_string(),
                            ctx,
                            msg,
                        )
                        .await;
                        return;
                    }
                }
            }
        }
        //when we are here some kind of Error occurred
        say_something("Some kind of Error occurred while trying to give you the Verified Role. This probably has to do something with permissions: Make sure the bot is over you in the Role hierarchy otherwise it can't assign you the roles.".to_string(), ctx, msg).await;
        return;
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("Connected as {}", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    dotenv().expect("please add a .env");
    let mut client = Client::builder(TOKEN.to_string())
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
