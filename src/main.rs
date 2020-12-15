mod command;
mod event_handler;
mod game;

use serenity::client::bridge::gateway::GatewayIntents;

#[tokio::main]
async fn main() {
    env_logger::init();

    let token =
        std::env::var("DISCORD_TOKEN").expect("Expected discord API token in `DISCORD_TOKEN`");

    let mut client = serenity::Client::builder(token)
        .type_map_insert::<game::GameModel>(game::GameModel::new())
        .event_handler(event_handler::Handler::new())
        .intents(GatewayIntents::GUILD_MESSAGE_REACTIONS | GatewayIntents::GUILD_MESSAGES)
        .await
        .expect("Error when building Client");

    match client.start().await {
        Ok(()) => println!("Exited with no errors"),
        Err(e) => eprintln!("Exited with error: {:?}", e),
    }
}
