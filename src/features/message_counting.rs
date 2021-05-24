use redis::{Commands, ErrorKind, RedisResult};
use serenity::model::guild::Member;

use crate::REDIS_CLIENT;

pub async fn handle_messages_redis(member: Member) {
    let guild_id = member.guild_id.to_string();
    let member_id = member.user.id.to_string();
    let con = REDIS_CLIENT.get_connection();
    if con.is_err() {
        println!("Error while opening redis connection");
        return;
    }
    let mut con = con.unwrap();
    let key = format!("verifybot:messages:{}", guild_id);
    //get current messages for this user
    let current_value: RedisResult<isize> = con.hget(&key, &member_id);
    let current_messages: i64;
    if current_value.is_err() {
        //TypeError means its probably nil so it can be set to 0, return on every other error
        if !(current_value.err().unwrap().kind() == ErrorKind::TypeError) {
            return;
        } else {
            current_messages = 0;
        }
    } else {
        current_messages = current_value.unwrap() as i64;
    }
    //save new message count to db
    redis::cmd("HSET")
        .arg(&key)
        .arg(&member_id)
        .arg(current_messages + 1)
        .execute(&mut con);
}
