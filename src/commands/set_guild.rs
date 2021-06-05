use crate::commands::command::Command;
use crate::say_something;
use crate::REDIS_CLIENT;
use serenity::async_trait;
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

        //get member object to check for perms
        let guild = msg.guild(&ctx).await;
        if guild.is_none() {
            send_err(ctx, msg).await;
            return;
        }

        let member = guild.clone().unwrap().member(&ctx.http, msg.author.id).await;
        if member.is_err() {
            send_err(ctx, msg).await;
            return;
        }
        let member = member.unwrap();
        let perms = member.permissions(&ctx).await;
        if perms.is_err() {
            send_err(ctx, msg).await;
            return;
        }
        //check for correct perms
        let perms = perms.unwrap();
        if !perms.manage_guild() {
            say_something(
                "You need the \"Manage Guild\" Permission to use this command".to_string(),
                ctx,
                msg,
            )
            .await;
            return;
        }

        let mut split = msg.content.splitn(2, ' ');
        let count = split.clone().count();
        if count != 2 {
            say_something(format!("Invalid usage: {} [name]", command), ctx, msg).await;
            return;
        }

        let _ = split.next().unwrap();
        let guild_name = split.next().unwrap().to_string();
        let res = save_to_db(guild.unwrap().id.to_string(), guild_name).await;
        if !res {
            send_err(ctx, msg).await;
            return;
        }
        say_something("Successfully set the new Guild name.".to_string(), ctx, msg).await;
    }
}
async fn save_to_db(guild_id: String, minecraft_guild_name: String) -> bool {
    let con = REDIS_CLIENT.get_connection();
    if con.is_err() {
        return false;
    }

    let key = format!("verifybot:config:{}", guild_id);
    redis::cmd("HSET")
        .arg(&key)
        .arg("minecraft_guild")
        .arg(&minecraft_guild_name)
        .execute(&mut con.unwrap());
    true
}

async fn send_err(ctx: Context, msg: Message) {
    say_something(
        "An Error occured while executing this command.".to_string(),
        ctx,
        msg,
    )
    .await;
}
