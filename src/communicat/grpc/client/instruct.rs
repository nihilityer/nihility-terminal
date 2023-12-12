use anyhow::Result;
use async_trait::async_trait;
use nihility_common::instruct::instruct_client::InstructClient;
use nihility_common::instruct::TextInstruct;
use nihility_common::response_code::RespCode;
use tokio::spawn;
use tokio::sync::mpsc::Receiver;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use tonic::Request;
use tracing::error;

use crate::communicat::SendInstructOperate;

#[async_trait]
impl SendInstructOperate for InstructClient<Channel> {
    async fn send_text(&mut self, instruct: TextInstruct) -> Result<RespCode> {
        Ok(self
            .send_text_instruct(Request::new(instruct))
            .await?
            .into_inner()
            .code())
    }

    async fn send_multiple_text(
        &mut self,
        instruct_stream: Receiver<TextInstruct>,
    ) -> Result<Receiver<RespCode>> {
        let (tx, rx) = tokio::sync::mpsc::channel::<RespCode>(128);
        let mut resp_stream = self
            .send_multiple_text_instruct(ReceiverStream::new(instruct_stream))
            .await?
            .into_inner();
        spawn(async move {
            while let Some(result) = resp_stream.next().await {
                match result {
                    Ok(resp) => {
                        match tx.send(resp.code()).await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("Instruct Grpc Client send_multiple_text Send To Core Error: {:?}", e);
                                break;
                            }
                        }
                    }
                    Err(status) => {
                        error!(
                            "Instruct Grpc Client send_multiple_text Send Error: {:?}",
                            &status
                        );
                        match tx.send(RespCode::UnknownError).await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("Instruct Grpc Client send_multiple_text Send To Core Error: {:?}", e);
                                break;
                            }
                        }
                    }
                }
            }
        });
        Ok(rx)
    }
}
