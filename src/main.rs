#[macro_use]
extern crate lazy_static;

mod calendar;
mod calendar_sync;
mod hash_map_vec;

use crate::calendar_sync::CalendarSync;
use crate::hash_map_vec::HashMapVec;
use chrono::Utc;
use serenity::async_trait;
use serenity::framework::StandardFramework;
use serenity::model::channel::Message;
use serenity::model::id::ChannelId;
use serenity::prelude::*;

const CONFIG_PATH: &str = "salle-bot.toml";

/// Given a message like `"<@mention> salles"` or `"salles <@mention> "`, returns `Some("salles")`.
/// If there is no command (just a mention), returns `""`. If the input doesn't match this format,
/// `None` is returned.
///
/// The function expects the message to contain a mention, and won't actually test for it.
fn get_command(message: &str) -> Option<&str> {
    let mut parts = message.split_whitespace();

    let first = parts.next()?;
    let second = parts.next();

    if parts.count() != 0 {
        return None;
    }

    // Mentions internally start with '<'
    if first.starts_with('<') {
        second.or(Some(""))
    } else {
        Some(first)
    }
}

enum Intent<'a> {
    /// View all loaded rooms
    Rooms,

    /// Find an empty room
    Find,

    /// Display the help message
    Help {
        /// If the help wasn't explicitly asked, which command was typed
        command: Option<&'a str>,
    },
}

impl<'a> Intent<'a> {
    async fn execute(
        &self,
        ctx: &Context,
        channel_id: ChannelId,
        cal: &CalendarSync,
    ) -> serenity::Result<Message> {
        match self {
            Self::Rooms => {
                let cal = cal.get();

                let bats: HashMapVec<_, _> = cal.rooms().into_iter().map(|room| (room.bat(), room)).collect();
                let bats_fields = bats.0
                    .into_iter()
                    .map(|(bat, rooms)| (format!("Bâtiment {}", bat), {
                        let mut s = String::new();
                        rooms.iter().for_each(|r| s.push_str(&r.to_string()));
                        s
                    }, false));

                channel_id.send_message(ctx, |m| {
                    m
                    .content("Voici ma base de données actuelle:")
                    .embed(|e| {
                        e
                            .fields(bats_fields)
                            .footer(|f| f.text("Dernière mise à jour le 32/13/2020 à 24:34")) // TODO
                    })
                }).await
            }
            Self::Find => channel_id.send_message(ctx, |m| {
                let cal = cal.get();

                let now = Utc::now();

                let mut rooms = cal.rooms_and_timetable();
                rooms.retain(|(_, tt)| tt.iter().all(|range| !range.contains(&now)));

                m.content(format!("Salles libres: {}", {
                    let mut s = String::new();
                    rooms.iter().for_each(|(r, _)| { s.push_str(&r.to_string()); s.push_str(", ") });
                    s
                }))
            }).await,
            Self::Help { command } => {
                channel_id.send_message(ctx, |m| {
                    if let Some(command) = command {
                        m.content(format!("Je ne connais pas la commande `{}`, jette un œil à celles que je supporte ⬇️", *command));
                    }

                    m.embed(|e| {
                        e.title("Aide")
                            .description("**Envoyer une commande:**\n> <@755016002546171995> <commande>\n**Commandes supportées:**")
                    })
                }).await
            }
        }
    }
}

struct Handler(CalendarSync);

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, new_message: Message) {
        if !new_message.author.bot
            && new_message.mentions.len() == 1
            && new_message.mentions_me(&ctx).await.unwrap_or_default()
        {
            let intent = match get_command(&new_message.content) {
                Some("") | Some("salle") | Some("cherche") => Intent::Find,
                Some("salles") => Intent::Rooms,
                Some("help") | Some("aide") => Intent::Help { command: None },
                Some(unknown) => Intent::Help {
                    command: Some(unknown),
                },
                None => return, // TODO handle every mention and send help if invalid
            };

            match intent.execute(&ctx, new_message.channel_id, &self.0).await {
                Ok(_) => (),
                Err(err) => eprintln!("error while sending message: {:?}", err),
            };
        }
    }
}

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN")
        .or_else(|_| std::fs::read_to_string("DISCORD_TOKEN"))
        .expect("discord token");

    if !std::fs::metadata(CONFIG_PATH).is_ok() {
        std::fs::write(CONFIG_PATH, include_str!("salle-bot-default.toml"))
            .expect("default config file cannot be written");
    }

    let config: toml::Value = std::fs::read_to_string(CONFIG_PATH)
        .expect("config file cannot be read (salle-bot.toml)")
        .parse()
        .expect("config file cannot be parsed");

    let calendar = CalendarSync::new(
        config
            .get("provider")
            .expect("provider is not set")
            .get("url")
            .expect("provider.url is not set")
            .as_str()
            .expect("provider.url is not a string")
            .into(),
    )
    .await;

    let mut client = Client::new(token.trim())
        .event_handler(Handler(calendar))
        .framework(StandardFramework::new())
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_command_empty() {
        assert_eq!(Some(""), get_command("<@mention> "));
    }

    #[test]
    fn get_command_first() {
        assert_eq!(Some("salles"), get_command("salles <@mention>"));
    }

    #[test]
    fn get_command_second() {
        assert_eq!(Some("salles"), get_command("<@mention> salles"));
    }

    #[test]
    fn get_command_bad() {
        assert_eq!(None, get_command("<@mention> a b c"));
        assert_eq!(None, get_command("a <@mention> b"));
    }
}
