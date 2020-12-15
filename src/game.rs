use crate::command::Command;
use futures::{
    FutureExt,
    stream::StreamExt,
};
use serenity::{collector::ReactionAction, model::prelude::*};
use serenity::prelude::*;
use std::{collections::hash_map::{Entry, HashMap}};

const DENY_CHALLENGE: char = '❌';
const ACCEPT_CHALLENGE: char = '✅';
const RED_PIECE: char = '🔴';
const YELLOW_PIECE: char = '🟡';
const BLANK_CELL: char = '⚫';

const NUMBER_EMOTES: &[&str] = &[
    "\u{31}\u{fe0f}\u{20e3}", // 1
    "\u{32}\u{fe0f}\u{20e3}", // 2
    "\u{33}\u{fe0f}\u{20e3}", // 3
    "\u{34}\u{fe0f}\u{20e3}", // 4
    "\u{35}\u{fe0f}\u{20e3}", // 5
    "\u{36}\u{fe0f}\u{20e3}", // 6
    "\u{37}\u{fe0f}\u{20e3}", // 7
    "\u{38}\u{fe0f}\u{20e3}", // 8
    "\u{39}\u{fe0f}\u{20e3}", // 9
];

pub struct GameModel {
    games: HashMap<ChannelId, tokio::sync::mpsc::Sender<GameAction>>,
}

impl TypeMapKey for GameModel {
    type Value = GameModel;
}

impl GameModel {
    pub fn new() -> Self {
        GameModel {
            games: HashMap::new(),
        }
    }

    /// Remove a game from the map of running games.
    async fn close_game(&mut self, channel_id: ChannelId) {
        self.games.remove(&channel_id);
    }

    /// Handles an incoming command.
    pub async fn run_command(&mut self, ctx: Context, command: Command) {
        match command {
            Command::Challenge {
                channel,
                challenger,
                opponent,
            } => self.handle_challenge(ctx, channel, challenger, opponent).await,
        }
    }

    /// Sends a message indicating that a challenge has been made, and spawns a task to handle this game.
    async fn handle_challenge(&mut self, ctx: Context, channel: ChannelId, challenger: User, opponent: User) {
        log::info!("Challenge from {} to {} on {}", challenger.id, opponent.id, channel);
        match self.games.entry(channel) {
            Entry::Occupied(_) => {
                let _ = channel
                    .send_message(&ctx, |msg| {
                        msg.content(format!(
                            "{} There is already a game in this channel.",
                            challenger.mention()
                        ))
                    })
                    .await;
            }

            Entry::Vacant(e) => {
                let mut challenge_message = channel
                    .send_message(&ctx, |msg| {
                        msg.content(format!(
                            "{} has been challenged to a game by {}!\n\nThis invite will expire in 60 seconds.",
                            opponent.mention(),
                            challenger.mention()
                        ))
                        .reactions([DENY_CHALLENGE, ACCEPT_CHALLENGE].iter().map(|c| c.clone()))
                    })
                    .await
                    .unwrap();

                let challenger_id = challenger.id;
                let opponent_id = opponent.id;
                let mut reaction_stream = challenge_message
                    .await_reactions(&ctx.shard)
                    .filter(move |r| {
                        r.user_id == Some(challenger_id) || r.user_id == Some(opponent_id)
                    })
                    .timeout(std::time::Duration::from_secs(60))
                    .await;

                let (send, recv) = tokio::sync::mpsc::channel(4);
                e.insert(send);

                let challenger_id = challenger.id;
                let opponent_id = opponent.id;
                let ctx = ctx.clone();
                tokio::spawn(async move {
                    let mut timed_out = true;
                    while let Some(reaction) = reaction_stream.next().await {
                        if reaction.is_added() {
                            let r = reaction.as_inner_ref();
                            if r.emoji == ReactionType::from(DENY_CHALLENGE) {
                                if r.user_id == Some(challenger_id) {
                                    let _ = challenge_message.edit(&ctx.http, |msg| {
                                        msg.content(format!(
                                            "{} has cancelled their challenge against {}",
                                            challenger_id.mention(),
                                            opponent_id.mention()
                                        ))
                                    })
                                    .await;
                                    reaction_stream.stop();
                                    timed_out = false;
                                    break
                                } else if r.user_id == Some(opponent_id) {
                                    let _ = challenge_message.edit(&ctx.http, |msg| {
                                        msg.content(format!(
                                            "{}'s challenge was declined by {}",
                                            challenger_id.mention(),
                                            opponent_id.mention()
                                        ))
                                    })
                                    .await;
                                    reaction_stream.stop();
                                    timed_out = false;
                                    break
                                }
                            } else if r.emoji == ReactionType::from(ACCEPT_CHALLENGE) && r.user_id == Some(opponent_id) {
                                let _ = challenge_message.edit(&ctx.http, |msg| {
                                    msg.content(format!(
                                        "{}'s challenge was accepted by {}!",
                                        challenger_id.mention(),
                                        opponent_id.mention()
                                    ))
                                })
                                .await;
                                reaction_stream.stop();
                                game(recv, ctx.clone(), channel, challenger, opponent).await;
                                timed_out = false;
                                break
                            }
                        }
                    }
                    if timed_out {
                        let _ = challenge_message.edit(&ctx.http, |msg| {
                            msg.content(format!(
                                "*{}'s challenge to {} has timed out.*",
                                challenger_id.mention(),
                                opponent_id.mention()
                            ))
                        })
                        .await;
                    }

                    // Remove this thread's channel from the `games` map
                    ctx.data.write().await.get_mut::<GameModel>().unwrap().close_game(channel).await;
                });
            }
        }
    }
}

/// Represents a game in progress
#[derive(Debug)]
struct GameState {
    reds_turn: bool,
    red_player: User,
    yellow_player: User,
    board: [[GameCell; 6]; 7],
    config: GameConfig
}

impl GameState {
    /// Writes out the game state as 
    fn message_content(&self) -> String {
        format!(
            "*Move timeout: {move_timeout} seconds*\n\
             {red_player}: {red_piece_emote}\n\
             {yellow_player}: {yellow_piece_emote}\n\n\
             {board}",
            
            move_timeout = self.config.move_timeout.as_secs(),
            red_player = self.red_player.mention(),
            yellow_player = self.yellow_player.mention(),
            red_piece_emote = RED_PIECE,
            yellow_piece_emote = YELLOW_PIECE,
            board = "",
        )
    }

    /// The current player places a piece in column 
    fn play_move(&mut self, column: usize) -> MoveOutcome {
        todo!()
    }
}

#[derive(Debug)]
struct GameConfig {
    /// Maximum time in-between moves before a game times out.
    move_timeout: std::time::Duration,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum GameCell {
    Empty, Red, Yellow
}

impl std::fmt::Display for GameCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            GameCell::Empty => BLANK_CELL,
            GameCell::Red => RED_PIECE,
            GameCell::Yellow => YELLOW_PIECE,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum MoveOutcome {
    RedWins,
    YellowWins,
    Draw,
    Continue,
    Illegal,
}

/// An action that can be sent to a game thread
pub enum GameAction {
    /// Forcefully end the game with a draw
    ForceDraw,
}

/// Handles a game in progress.
async fn game(mut recv: tokio::sync::mpsc::Receiver<GameAction>, ctx: Context, channel: ChannelId, challenger: User, opponent: User) {
    // This blocks but whatev
    let (red_player, yellow_player) = if rand::random::<bool>() {
        (challenger, opponent)
    } else {
        (opponent, challenger)
    };
    let move_timeout = std::time::Duration::from_secs(120);

    let mut game_state = GameState {
        config: GameConfig {
            move_timeout
        },

        red_player, yellow_player,
        reds_turn: true,

        board: [[GameCell::Empty; 6]; 7],
    };

    let mut board_message = channel.send_message(
        &ctx.http, 
        |msg|
            msg.content(game_state.message_content())
                .reactions(NUMBER_EMOTES.iter().take(7).map(|&s| {
                    ReactionType::Unicode(String::from(s))
                }))
    ).await.unwrap();

    loop {
        let current_player_id = if game_state.reds_turn {game_state.red_player.id} else {game_state.yellow_player.id};
        let react_watch = board_message
            .await_reaction(&ctx.shard)
            .author_id(current_player_id)
            .filter(|r| {
                if let ReactionType::Unicode(e) = &r.emoji {
                    NUMBER_EMOTES[..7].contains(&&e[..])
                } else { false }
            })
            .timeout(game_state.config.move_timeout);

        tokio::select! {
            act = recv.recv() => match act {
                Some(GameAction::ForceDraw) | None => break, // Game forcefully closed prematurely
            },
            r = react_watch => match r {
                None => break, // Timeout
                Some(r) => {
                    let r = r.as_inner_ref();
                    let emoji = match &r.emoji {
                        ReactionType::Unicode(e) => e, _ => unreachable!()
                    };
                    let col = NUMBER_EMOTES
                        .iter()
                        .position(|e| {
                            e == &&emoji[..]
                        })
                        .unwrap();
                    match game_state.play_move(col) {
                        MoveOutcome::RedWins => todo!(),
                        MoveOutcome::YellowWins => todo!(),
                        MoveOutcome::Draw => todo!(),
                        MoveOutcome::Continue | MoveOutcome::Illegal => {}
                    };

                    // TODO: Detect if `Manage Messages` is enabled
                    let _ = r.delete(&ctx.http).await;

                    board_message.edit(&ctx.http, |msg|
                        msg.content(game_state.message_content())
                    ).await.unwrap();
                },
                _ => unreachable!()
            }
        };
    }
}