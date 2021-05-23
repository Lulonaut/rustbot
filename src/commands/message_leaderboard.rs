use std::cmp::Reverse;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use serenity::async_trait;
use serenity::client::Context;
use serenity::model::channel::Message;
use string_builder::Builder;

use crate::commands::command::Command;
use crate::say_something;

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

        let path = Path::new("messages.json");
        if !path.exists() {
            say_something("No data found.".to_string(), ctx, msg).await;
            return;
        }

        let mut file = File::open(path).unwrap();
        let mut contents = String::new();
        if let Err(_) = file.read_to_string(&mut contents) {
            send_err(ctx, msg).await;
            return;
        }
        //parse contents

        let json = json::parse(contents.as_str());
        if json.is_err() {
            send_err(ctx, msg).await;
            return;
        }

        let json = json.unwrap();

        //check if this guild has data

        if msg.guild_id.is_none() {
            send_err(ctx, msg).await;
            return;
        }
        let guild_id = msg.guild_id.unwrap().to_string();

        if !json.has_key("messages") {
            say_something(
                "There are currently no messages stored for this Server.".to_string(),
                ctx,
                msg,
            )
            .await;
            return;
        }
        let json = &json["messages"];
        if !json.has_key(guild_id.as_str()) {
            say_something(
                "There are currently no messages stored for this Server.".to_string(),
                ctx,
                msg,
            )
            .await;
            return;
        }

        //put all messages in map
        let guild_messages = &json[guild_id.as_str()];
        let mut messages: HashMap<u64, String> = HashMap::new();

        for (key, value) in guild_messages.entries() {
            let key = key.clone().to_string();
            let value = value.as_u64();
            if value.is_none() {
                continue;
            }
            messages.insert(value.unwrap(), key);
        }
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
