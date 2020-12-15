use serenity::{async_trait, client::EventHandler, model::prelude::*, prelude::*};

use crate::command::ParseCommandError;

pub struct Handler;

impl Handler {
    pub fn new() -> Self {
        Handler
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, new_message: Message) {
        // log::info!(
        //     "MESSAGE ({} on {}): {}",
        //     new_message.author.id,
        //     new_message
        //         .guild_id
        //         .map(|id| format!("{}", id))
        //         .unwrap_or(String::from("DM")),
        //     new_message.content
        // );

        use crate::command;
        match command::parse_command(&new_message) {
            Ok(command) => {
                tokio::spawn(async move {
                    let mut data = ctx.data.write().await;
                    let game = data.get_mut::<crate::game::GameModel>().unwrap();
                    game.run_command(ctx.clone(), command).await
                });
            }

            Err(e) => match e {
                ParseCommandError::BotAuthor | ParseCommandError::NoPrefix => {}

                ParseCommandError::InvalidTargetUser => {
                    let _ = new_message
                        .reply(
                            &ctx.http,
                            "That user could not be found or was not specified.",
                        )
                        .await;
                }

                ParseCommandError::InvalidCommand(_) => {
                    let _ = new_message.react(&ctx.http, '❓').await.unwrap();
                }
            },
        }
    }
}
