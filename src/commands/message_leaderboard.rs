use std::fs::File;
use std::io::Read;
use std::path::Path;

use serenity::async_trait;
use serenity::client::Context;
use serenity::model::channel::Message;

use crate::commands::command::Command;
use crate::say_something;

pub struct CommandArgs {
    pub prefix: String,
    pub command: String,
}

#[async_trait]
impl Command for CommandArgs {
    async fn execute(&self, ctx: &Context, msg: &Message) {
        if !msg
            .content
            .starts_with(format!("{}{}", self.prefix, self.command).as_str())
        {
            return;
        }
        let ctx = ctx.clone();
        let msg = msg.clone();
        //read contents of the file

        let path = Path::new("messages.json");
        if !path.exists() {
            say_something("No data found.".to_string(), ctx, msg);
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

        let guild_messages = &json[guild_id.as_str()];
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
