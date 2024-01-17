use tokio::sync::mpsc;
use tokio::{select, signal};
use tracing::info;

use nihility_terminal::NihilityTerminal;

#[tokio::main]
pub async fn main() {
    println!(
        r#"      ___                       ___                       ___                   ___           ___           ___           ___
     /\__\          ___        /\__\          ___        /\__\      ___        /\  \         |\__\         /\  \         /\  \
    /::|  |        /\  \      /:/  /         /\  \      /:/  /     /\  \       \:\  \        |:|  |       /::\  \       /::\  \
   /:|:|  |        \:\  \    /:/__/          \:\  \    /:/  /      \:\  \       \:\  \       |:|  |      /:/\:\  \     /:/\:\  \
  /:/|:|  |__      /::\__\  /::\  \ ___      /::\__\  /:/  /       /::\__\      /::\  \      |:|__|__   /::\~\:\  \   /::\~\:\  \
 /:/ |:| /\__\  __/:/\/__/ /:/\:\  /\__\  __/:/\/__/ /:/__/     __/:/\/__/     /:/\:\__\     /::::\__\ /:/\:\ \:\__\ /:/\:\ \:\__\
 \/__|:|/:/  / /\/:/  /    \/__\:\/:/  / /\/:/  /    \:\  \    /\/:/  /       /:/  \/__/    /:/~~/~    \:\~\:\ \/__/ \/_|::\/:/  /
     |:/:/  /  \::/__/          \::/  /  \::/__/      \:\  \   \::/__/       /:/  /        /:/  /       \:\ \:\__\      |:|::/  /
     |::/  /    \:\__\          /:/  /    \:\__\       \:\  \   \:\__\       \/__/         \/__/         \:\ \/__/      |:|\/__/
     /:/  /      \/__/         /:/  /      \/__/        \:\__\   \/__/                                    \:\__\        |:|  |
     \/__/                     \/__/                     \/__/                                             \/__/         \|__|    "#
    );
    let (shutdown_se, mut shutdown_re) = mpsc::channel::<String>(4);
    let cancellation_token = NihilityTerminal::get_cancellation_token();
    NihilityTerminal::set_close_sender(shutdown_se.downgrade());
    if let Err(e) = NihilityTerminal::start().await {
        println!("{:?}", e);
    }
    drop(shutdown_se);
    select! {
        _ = signal::ctrl_c() => {
            cancellation_token.cancel();
        },
        _ = cancellation_token.cancelled() => {}
    }
    while let Some(module_name) = shutdown_re.recv().await {
        info!("{} Exit", module_name);
    }
    println!("press any key to exit");
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
}
