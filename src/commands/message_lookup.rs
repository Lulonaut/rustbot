use serenity::async_trait;
use serenity::client::Context;
use serenity::model::channel::Message;

use crate::commands::command::Command;

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
    }
}
