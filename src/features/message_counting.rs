use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use json::JsonValue;
use serenity::model::guild::Member;

pub async fn handle_messages(member: Member) {
    //make sure file exists
    let path = Path::new("messages.json");
    let exists = path.exists();
    //create file if it doesn't exist
    if !exists {
        let file = File::create(path);
        if file.is_err() {
            println!("Error while creating messages file.");
            return;
        }
        if let Err(_) = file.unwrap().write_all("{}".as_bytes()) {
            println!("Error while writing to file");
            return;
        }
    }
    //file exists now, getting content
    let mut file = File::open(path).unwrap();
    let mut contents = String::new();
    if let Err(_) = file.read_to_string(&mut contents) {
        println!("Error while reading file");
        return;
    }

    //parsing content
    let json = json::parse(contents.as_str());
    if json.is_err() {
        println!("Error while parsing JSON");
        return;
    }

    let mut json = json.unwrap();
    //enter messages
    if !json.has_key("messages") {
        json["messages"] = JsonValue::new_object();
    }
    let messages = &mut json["messages"];

    //enter guildId
    let guild_id = member.guild_id.as_u64();
    if !messages.has_key(guild_id.to_string().as_str()) {
        messages[guild_id.to_string().as_str()] = JsonValue::new_object();
    }

    let guild = &mut messages[guild_id.to_string().as_str()];
    //check if userId is already present
    let member_id = member.user.id.as_u64().to_string();
    if guild.has_key(member_id.as_str()) {
        //increment messages by one
        guild[member_id.as_str()] = JsonValue::from(guild[member_id.as_str()].as_i8().unwrap() + 1);
    } else {
        //add user to guild and set messages to 1
        guild[member_id.as_str()] = JsonValue::from(1);
    }

    let mut file = File::create(path).unwrap();
    if let Err(err) = file.write_all(json.dump().to_string().as_bytes()) {
        println!("Error while writing file. {}", err);
        return;
    }
}
