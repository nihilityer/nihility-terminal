use async_trait::async_trait;
use nihility_common::module_info::ModuleInfoReq;

#[cfg(unix)]
pub struct PipeProcessor;

#[cfg(unix)]
impl PipeProcessor {
    pub async fn start(
        pipe_config: &PipeConfig,
        module_sender: Sender<Submodule>,
        instruct_sender: Sender<InstructEntity>,
        manipulate_sender: Sender<ManipulateEntity>,
    ) -> Result<(), AppError> {
        if !pipe_config.enable {
            return Ok(());
        }
        tracing::debug!("PipeProcessor start!");
        if !Path::try_exists(&pipe_config.unix.directory.as_ref())? {
            fs::create_dir_all(&pipe_config.unix.directory)?;
        }
        tracing::debug!("pipe work dir: {}", &pipe_config.unix.directory);

        let module_path = format!(
            "{}/{}",
            pipe_config.unix.directory.as_str(),
            pipe_config.unix.module.to_string()
        );
        let instruct_path = format!(
            "{}/{}",
            pipe_config.unix.directory.as_str(),
            pipe_config.unix.instruct_receiver.to_string()
        );
        let manipulate_path = format!(
            "{}/{}",
            pipe_config.unix.directory.as_str(),
            pipe_config.unix.manipulate_receiver.to_string()
        );

        if !Path::try_exists(&module_path.as_ref())? {
            tracing::debug!(
                "cannot found model pipe file, try create on {}",
                &module_path
            );
            Command::new("mkfifo").arg(&module_path).output()?;
        }
        if !Path::try_exists(&instruct_path.as_ref())? {
            tracing::debug!(
                "cannot found instruct pipe receiver file, try create on {}",
                &instruct_path
            );
            Command::new("mkfifo").arg(&instruct_path).output()?;
        }
        if !Path::try_exists(&manipulate_path.as_ref())? {
            tracing::debug!(
                "cannot found manipulate pipe receiver file, try create on {}",
                &manipulate_path
            );
            Command::new("mkfifo").arg(&manipulate_path).output()?;
        }

        tracing::debug!("start create pipe from file");
        let module_rx = OpenOptions::new().open_receiver(&module_path)?;
        tracing::debug!("model pipe create success");
        let instruct_rx = OpenOptions::new().open_receiver(&instruct_path)?;
        tracing::debug!("instruct pipe create success");
        let manipulate_rx = OpenOptions::new().open_receiver(&manipulate_path)?;
        tracing::debug!("manipulate pipe create success");

        loop {
            Self::module_pipe_processor(&module_sender, &module_rx).await?;
            Self::instruct_pipe_processor(&instruct_sender, &instruct_rx).await?;
            Self::manipulate_pipe_processor(&manipulate_sender, &manipulate_rx).await?;
        }
    }

    /// 负责接收pipe模块注册信息
    async fn module_pipe_processor(
        module_sender: &Sender<Submodule>,
        module_rx: &Receiver,
    ) -> Result<(), AppError> {
        module_rx.readable().await?;
        let mut msg = vec![0; 1024];

        match module_rx.try_read(&mut msg) {
            Ok(n) => {
                if n > 0 {
                    let result: ModuleInfoReq = ModuleInfoReq::decode(&msg[..n])?;
                    tracing::debug!("pipe model name:{:?}", &result.name);
                    let model = Submodule::create_by_req(result).await?;
                    module_sender.send(model).await?;
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(_) => {
                return Err(AppError::PipeError("model".to_string()));
            }
        }
        Ok(())
    }

    /// 负责接收pipe指令信息
    async fn instruct_pipe_processor(
        instruct_sender: &Sender<InstructEntity>,
        instruct_rx: &Receiver,
    ) -> Result<(), AppError> {
        instruct_rx.readable().await?;
        let mut data = vec![0; 1024];

        match instruct_rx.try_read(&mut data) {
            Ok(n) => {
                if n > 0 {
                    let result: InstructReq = InstructReq::decode(&data[..n])?;
                    let instruct_entity = InstructEntity::create_by_req(result);
                    tracing::debug!("pipe instruct: {:?}", &instruct_entity);
                    instruct_sender.send(instruct_entity).await?;
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(_) => {
                return Err(AppError::PipeError("model".to_string()));
            }
        }
        Ok(())
    }

    /// 负责接收pipe操作信息
    async fn manipulate_pipe_processor(
        manipulate_sender: &Sender<ManipulateEntity>,
        manipulate_rx: &Receiver,
    ) -> Result<(), AppError> {
        manipulate_rx.readable().await?;
        let mut data = vec![0; 1024];

        match manipulate_rx.try_read(&mut data) {
            Ok(n) => {
                if n > 0 {
                    let result: ManipulateReq = ManipulateReq::decode(&data[..n])?;
                    let manipulate_entity = ManipulateEntity::create_by_req(result);
                    tracing::debug!("pipe manipulate: {:?}", &manipulate_entity);
                    manipulate_sender.send(manipulate_entity).await?;
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(_) => {
                return Err(AppError::PipeError("model".to_string()));
            }
        }
        Ok(())
    }
}

#[cfg(unix)]
#[async_trait]
impl SendInstructOperate for PipeUnixInstructClient {
    async fn send(&mut self, instruct: InstructReq) -> Result<RespCode, AppError> {
        let result = self.send_instruct(instruct).await?;
        Ok(result.status)
    }
}

#[cfg(unix)]
#[async_trait]
impl SendManipulateOperate for PipeUnixManipulateClient {
    async fn send(&mut self, manipulate: ManipulateReq) -> Result<RespCode, AppError> {
        let result = self.send_manipulate(manipulate).await?;
        Ok(result.status)
    }
}

#[cfg(unix)]
pub struct PipeUnixInstructClient {
    pub instruct_sender: Sender,
}

#[cfg(unix)]
pub struct PipeUnixManipulateClient {
    pub manipulate_sender: Sender,
}

#[cfg(unix)]
impl PipeUnixInstructClient {
    pub fn init(path: String) -> Result<Self, AppError> {
        tracing::debug!("open instruct pipe sender from {}", &path);
        let sender = OpenOptions::new().open_sender(path)?;
        Ok(PipeUnixInstructClient {
            instruct_sender: sender,
        })
    }

    pub async fn send_instruct(&self, instruct_req: InstructReq) -> Result<InstructResp, AppError> {
        loop {
            self.instruct_sender.writable().await?;
            let mut data = vec![0; 1024];
            instruct_req.encode(&mut data)?;

            match self.instruct_sender.try_write(&data) {
                Ok(_) => {
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        Ok(InstructResp { status: true })
    }
}

#[cfg(unix)]
impl PipeUnixManipulateClient {
    pub fn init(path: String) -> Result<Self, AppError> {
        tracing::debug!("open manipulate pipe sender from {}", &path);
        let sender = OpenOptions::new().open_sender(path)?;
        Ok(PipeUnixManipulateClient {
            manipulate_sender: sender,
        })
    }

    pub async fn send_manipulate(
        &self,
        manipulate_req: ManipulateReq,
    ) -> Result<ManipulateResp, AppError> {
        loop {
            self.manipulate_sender.writable().await?;
            let mut data = vec![0; 1024];
            manipulate_req.encode(&mut data)?;

            match self.manipulate_sender.try_write(&data) {
                Ok(_) => {
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        Ok(ManipulateResp { status: true })
    }
}
