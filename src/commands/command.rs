use serenity::async_trait;
use serenity::client::Context;
use serenity::model::channel::Message;

#[async_trait]
pub trait Command {
    async fn execute(&self, ctx: &Context, msg: &Message);
}
