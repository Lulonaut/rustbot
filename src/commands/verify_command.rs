use std::time::Duration;

use redis::Commands;
use redis::ErrorKind;
use redis::RedisResult;
use serde_json::Value;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::guild::Role;
use serenity::prelude::*;

use crate::commands::command::Command;
use crate::commands::verify_command::PossibleErrors::{
    DiscordNotLinked, HypixelAPIError, InvalidUsername, MojangAPIError,
};
use crate::say_something;
use crate::REDIS_CLIENT;

pub struct VerifyCommandArgs {
    pub prefix: String,
    pub command: String,
    pub api_key: String,
    pub role_name: String,
}

#[async_trait]
impl Command for VerifyCommandArgs {
    async fn execute(&self, ctx: &Context, msg: &Message) {
        let command = format!("{}{}", self.prefix, self.command);
        if !msg.content.starts_with(&command) {
            return;
        }
        let ctx = ctx.clone();
        let msg = msg.clone();

        //check for correct usage
        let args = msg.content.split(' ');
        if args.count() != 2 {
            say_something(
                format!("Invalid usage: `{}verify Username`", self.prefix),
                ctx,
                msg,
            )
            .await;
            return;
        }
        //get username
        let mut iter = msg.content.splitn(2, ' ');
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
        let api_key = &self.api_key;
        let api_key = api_key.clone();
        let info = get_info(String::from(username), api_key).await;
        if info.is_err() {
            let error = info.err().unwrap();
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

        let discord = info.ok().unwrap();
        let linked_discord = discord.discord;
        let rank = discord.rank;
        let username = discord.username;
        let user_guild = discord.guild;

        let mut discriminator: String = msg.author.discriminator.to_string();
        if discriminator.len() == 1 {
            discriminator = format!("000{}", discriminator)
        }

        if discriminator.len() == 2 {
            discriminator = format!("00{}", discriminator);
        }

        if discriminator.len() == 3 {
            discriminator = format!("0{}", discriminator);
        }

        let user_discord: String = msg.author.name.to_string() + "#" + discriminator.as_str();

        if linked_discord != user_discord {
            say_something(format!("The linked Username `{}` doesn't match your Discord Username: `{}`. If you just changed this wait a bit and try again.", linked_discord, user_discord), ctx, msg).await;
            return;
        }
        //assign Verified role
        let member = msg.member(&ctx).await;
        if member.is_err() {
            say_something("There was an Error while fetching your profile from the Discord API and therefore the bot can't assign you the roles. Please try again later".to_string(), ctx, msg).await;
            return;
        }
        let member = &mut member.unwrap();
        let member2 = &mut member.clone();

        if let Some(guild_id) = msg.guild_id {
            if let Some(guild) = guild_id.to_guild_cached(&ctx).await {
                if let Some(role_id) = guild.role_by_name(&*self.role_name) {
                    if let Err(err) = &member.add_role(&ctx, role_id).await {
                        println!("error while adding role {}", err);
                    } else {
                        //add new rank role and remove existing ones
                        let mut member = member.clone();
                        let current_roles = &mut member.roles;
                        let role = get_rank_role(rank, &ctx, &msg).await;
                        if role.is_none() {
                            say_something(
                                "Error while getting Rank roles, maybe they dont exist?"
                                    .to_string(),
                                ctx,
                                msg,
                            )
                            .await;
                            return;
                        }
                        let rank_role = role.unwrap();

                        for i in current_roles {
                            let i = &*i;

                            if let Some(role) = i.to_role_cached(&ctx).await {
                                if rank_role == role {
                                    continue;
                                }
                                //remove existing rank roles
                                if role.name == *"VIP"
                                    || role.name == *"VIP+"
                                    || role.name == *"MVP"
                                    || role.name == *"MVP+"
                                    || role.name == *"MVP++"
                                {
                                    if let Err(_) = &mut member2.remove_role(&ctx, i).await {
                                        say_something("Some kind of Error occurred while trying to give you the role for your Rank. This probably has to do something with permissions: Make sure the bot is over you in the Role hierarchy otherwise it can't assign you the roles.".to_string(), ctx, msg).await;
                                        return;
                                    }
                                }
                            }
                        }
                        //add current rank role
                        if let Err(_) = member.add_role(&ctx, rank_role.id).await {
                            say_something("Some kind of Error occurred while trying to give you the role for your Rank. This probably has to do something with permissions: Make sure the bot is over you in the Role hierarchy otherwise it can't assign you the roles. Also make sure the roles exist.".to_string(), ctx, msg).await;
                            return;
                        }

                        //change username
                        if let Err(_) = member.edit(&ctx, |m| m.nickname(username)).await {
                            say_something("The bot was unable to change your nickname. This probably has to do something with permissions: Make sure the bot is over you in the Role hierarchy otherwise it can't assign change your nickname.".to_string(), ctx, msg).await;
                            return;
                        }
                        //check if guild matches
                        let con = REDIS_CLIENT.get_connection();
                        if con.is_err() {
                            say_something("Some error occured while trying to get Hypixel Guild Info but you should still have the roles.".to_string(), ctx, msg).await;
                            return;
                        }
                        let mut con = con.unwrap();

                        let key = format!("verifybot:config:{}", guild_id);
                        let guild_stored: RedisResult<String> = con.hget(key, "minecraft_guild");
                        if guild_stored.is_err() {
                            //TypeError means its probably nil
                            if guild_stored.err().unwrap().kind() != ErrorKind::TypeError {
                                say_something("An Error occured while trying to get the Minecraft Guild set for this Server but you should still have the roles".to_string(), ctx, msg).await;
                                return;
                            }
                        } else if guild_stored.unwrap() == user_guild {
                            if let Some(role_id) = guild.role_by_name("Guild Member") {
                                if let Err(_) = member.add_role(&ctx, role_id).await {
                                    say_something("There as an Error assigning you the Guild Member role but you should still have the other roles".to_string(), ctx, msg).await;
                                    return;
                                }
                            } else {
                                say_something("There was an Error retreiving the Guild Member role but you should still have the other roles".to_string(), ctx, msg).await;
                                return;
                            }
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
        say_something("Some Error occurred while trying to give you the Verified Role. This probably has to do something with permissions: Make sure the bot is over you in the Role hierarchy otherwise it can't assign you the roles.".to_string(), ctx, msg).await;
        return;
    }
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
    Vip,
    Vipplus,
    Mvp,
    Mvpplus,
    Mvpplusplus,
}

struct ApiInfo {
    discord: String,
    rank: HypixelRanks,
    username: String,
    guild: String,
}

async fn get_rank_role(rank: HypixelRanks, ctx: &Context, msg: &Message) -> Option<Role> {
    let rank_string;
    match rank {
        HypixelRanks::Mvpplusplus => rank_string = "MVP++",
        HypixelRanks::Mvpplus => rank_string = "MVP+",
        HypixelRanks::Mvp => rank_string = "MVP",
        HypixelRanks::Vipplus => rank_string = "VIP+",
        HypixelRanks::Vip => rank_string = "VIP",
        HypixelRanks::Default => return None,
    }
    if let Some(guild_id) = msg.guild_id {
        if let Some(guild) = guild_id.to_guild_cached(&ctx).await {
            if let Some(role_id) = guild.role_by_name(rank_string) {
                return Some(role_id.clone());
            }
        }
    }
    None
}

async fn get_guild(player_uuid: String, api_key: String) -> String {
    let url = format!(
        "https://api.hypixel.net/guild?key={}&player={}",
        api_key, player_uuid
    );

    let client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(20))
        .build()
        .unwrap();

    let response = client.get(url).send().await;

    if response.is_err() {
        return "".to_string();
    }

    let json = serde_json::from_str(response.unwrap().text().await.unwrap().as_str());
    if json.is_err() {
        return "".to_string();
    }
    let json: Value = json.expect("");
    if let Some(guild) = json.get("guild") {
        if let Some(name) = guild.get("name") {
            return name.as_str().unwrap().to_string();
        }
    }

    "".to_string()
}

fn get_rank(api_response: &Value) -> HypixelRanks {
    //based on my API Wrapper in Java: https://github.com/Lulonaut/HypixelAPIWrapper/blob/d43c73c00f2bc111cf407c6a325cb686a3ec899a/src/main/java/de/lulonaut/wrapper/utils/getStuff.java#L19
    let player = api_response.get("player").unwrap();

    //check for owner and other weird ranks (eg: Technoblade Pig rank)
    if player.get("prefix").is_some() {
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
            return HypixelRanks::Mvpplusplus;
        }
    }
    if let Some(rank) = player.get("newPackageRank") {
        let rank = rank.as_str().unwrap();
        match rank {
            "MVP_PLUS" => return HypixelRanks::Mvpplus,
            "MVP" => return HypixelRanks::Mvp,
            "VIP_PLUS" => return HypixelRanks::Vipplus,
            "VIP" => return HypixelRanks::Vip,
            _ => {}
        }
    }

    HypixelRanks::Default
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

async fn get_info(username: String, api_key: String) -> Result<ApiInfo, PossibleErrors> {
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
    if json.get("error").is_some() {
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
            api_key, uuid
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
    if json.get("player").is_some() {
    } else {
        return Err(HypixelAPIError);
    }
    let rank = get_rank(&json);
    let username = get_username(&json);
    let guild = get_guild(uuid, api_key).await;

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
                        guild,
                    });
                }
            }
        }
    } else {
        return Err(HypixelAPIError);
    }
    Err(DiscordNotLinked)
}
