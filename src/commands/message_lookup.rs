use redis::Commands;
use redis::ErrorKind;
use redis::RedisResult;
use serenity::async_trait;

use crate::commands::command::Command;
use crate::say_something;
use crate::REDIS_CLIENT;
use serenity::client::Context;
use serenity::model::channel::Message;

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

        let user_id: String;

        let split = msg.content.split(" ");
        if split.count() == 2 {
            let mut iter = msg.content.splitn(2, " ");
            let _ = iter.next().unwrap();
            let user = iter.next().unwrap().to_string();
            //Message contains "&" if a role was mentioned
            if user.contains("&") {
                say_something("Invalid User".to_string(), ctx, msg).await;
                return;
            }
            user_id = user.replace("<@!", "").replace(">", "");
        } else {
            user_id = msg.author.id.to_string();
        }

        let guild_id = msg.guild_id.unwrap().to_string();
        let key = format!("verifybot:messages:{}", guild_id);
        //make query
        let con = REDIS_CLIENT.get_connection();
        if con.is_err() {
            send_err(ctx, msg).await;
            return;
        }
        let mut con = con.unwrap();
        let result: RedisResult<isize> = con.hget(key, &user_id);
        let current_messages: isize;
        if result.is_err() {
            if !(result.err().unwrap().kind() == ErrorKind::TypeError) {
                send_err(ctx, msg).await;
                return;
            } else {
                current_messages = 0;
            }
        } else {
            current_messages = result.unwrap();
        }

        let message = format!(
            "<@!{}> currently has {} messages.",
            user_id, current_messages
        );
        //send embed
        let _ = msg
            .channel_id
            .send_message(&ctx.http, |m| {
                m.embed(|e| e.title("Message lookup").description(message));
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
