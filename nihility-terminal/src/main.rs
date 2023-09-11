use nihility_terminal::{AppError, NihilityTerminal};

#[tokio::main]
pub async fn main() -> Result<(), AppError> {
    NihilityTerminal::start().await?;
    Ok(())
}
