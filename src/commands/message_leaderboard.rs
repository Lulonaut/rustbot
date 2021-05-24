use std::cmp::Reverse;
use std::collections::HashMap;

use serenity::async_trait;
use serenity::client::Context;
use serenity::model::channel::Message;
use string_builder::Builder;

use crate::commands::command::Command;
use crate::say_something;
use crate::REDIS_CLIENT;

pub struct CommandArgs {
    pub prefix: String,
    pub command: String,
}

#[async_trait]
impl Command for CommandArgs {
    async fn execute(&self, ctx: &Context, msg: &Message) {
        let command = format!("{}{}", self.prefix, self.command);
        if !msg.content.starts_with(&command) {
            return;
        }
        let ctx = ctx.clone();
        let msg = msg.clone();
        //read contents of the file
        let messages = get_messages(*msg.guild_id.unwrap().as_u64()).await;
        if messages.is_none() {
            send_err(ctx, msg).await;
            return;
        }
        let messages = messages.unwrap();
        //sort messages in Vector
        let mut sorted: Vec<_> = messages.iter().collect();
        sorted.sort_by_key(|w| Reverse(*w));

        //assemble final message
        let mut builder = Builder::default();
        let mut current_position: u8 = 1;
        for (key, value) in sorted.iter() {
            //discord mention
            let output = format!(
                "<@!{}> has {} messages and is Place {}\n",
                value, key, current_position
            );
            builder.append(output);
            if current_position == 10 {
                break;
            }
            current_position += 1;
        }
        if builder.len() == 0 {
            say_something(
                "There are currently no messages stored for this Server.".to_string(),
                ctx,
                msg,
            )
            .await;
            return;
        }
        let message = builder.string().unwrap();
        let _ = msg
            .channel_id
            .send_message(&ctx.http, |m| {
                m.embed(|e| e.title("Current message leaderboard").description(message));
                m
            })
            .await;
    }
}

async fn send_err(ctx: Context, msg: Message) {
    say_something(
        "An internal Error occurred while processing this command.".to_string(),
        ctx,
        msg,
    )
    .await;
}

async fn get_messages(guild_id: u64) -> Option<HashMap<u64, String>> {
    let con = REDIS_CLIENT.get_connection();
    if con.is_err() {
        println!("Error while opening redis connection");
        return None;
    }
    let mut con = con.unwrap();
    let key = format!("verifybot:messages:{}", guild_id);
    let result = redis::cmd("HGETALL").arg(&key).clone().iter(&mut con);
    if result.is_err() {
        println!("err {:?}", result.err());
        return None;
    }

    let result: redis::Iter<String> = result.unwrap();
    let mut messages: HashMap<u64, String> = HashMap::new();
    let mut key = true;
    let mut current_key: String = "".to_string();
    for x in result {
        if key {
            current_key = x;
            key = false;
        } else {
            messages.insert(x.parse().unwrap(), current_key.clone());
            key = true;
        }
    }

    return Some(messages);
}
