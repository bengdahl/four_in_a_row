use serenity::model::prelude::*;

/// Represents ways a message may fail to be parsed as a valid command
#[derive(Debug, PartialEq, Clone)]
pub enum ParseCommandError {
    /// This message does not have the bot's prefix
    NoPrefix,
    /// The author of this message is a bot
    BotAuthor,
    /// The target of this command could not be found or was not specified
    InvalidTargetUser,
    /// This command doesn't exist or is malformed
    InvalidCommand(String),
}

/// Represents a command sent by a user
#[derive(Debug, PartialEq, Clone)]
pub enum Command {
    Challenge {
        channel: ChannelId,
        challenger: User,
        opponent: User,
    },
}

pub fn parse_command(msg: &Message) -> Result<Command, ParseCommandError> {
    if !msg.content.starts_with("c4!") {
        return Err(ParseCommandError::NoPrefix);
    }
    if msg.author.bot {
        return Err(ParseCommandError::BotAuthor);
    }

    let command_name = msg
        .content
        .get("c4!".len()..)
        .unwrap() // We already confirmed that the message has the prefix
        .split_ascii_whitespace()
        .next();

    match command_name {
        Some("challenge") => {
            let channel = msg.channel_id;
            let challenger = msg.author.clone();
            let opponent = msg
                .mentions
                .get(0)
                .ok_or(ParseCommandError::InvalidTargetUser)?
                .clone();

            Ok(Command::Challenge {
                channel,
                challenger,
                opponent,
            })
        }

        s => Err(ParseCommandError::InvalidCommand(String::from(
            s.unwrap_or(""),
        ))),
    }
}
