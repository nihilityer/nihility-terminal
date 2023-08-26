use std::error::Error;
use nihility_terminal::NihilityTerminal;
use nihility_terminal::SummaryConfig;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    NihilityTerminal::init(SummaryConfig::default()).await?
        .run().await?;
    Ok(())
}
