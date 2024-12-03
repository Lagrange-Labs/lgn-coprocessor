use anyhow::Result;
use protobuf::worker_to_gw_request;
use protobuf::WorkerToGwRequest;
use protobuf::WorkerToGwResponse;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tonic::metadata::MetadataValue;
use tonic::Request;
use tracing::error;
use tracing::info;

pub mod test;

pub mod protobuf
{
    tonic::include_proto!("lagrange");
}
const MAX_GRPC_MESSAGE_SIZE_MB: usize = 16;

pub struct GrpcConfig
{
    /// Gateway URI
    pub gateway: String,
    /// Version of the worker to advertise to the GW
    pub version: String,
    /// Token authorization to put in bearer attribute
    pub token: String,
    /// worker class to advertise to the Gateway
    /// TODO: make that a vector to specify multiple classes
    pub class: String,
    pub max_grpc_message_size_mb: Option<usize>,
}

impl GrpcConfig
{
    pub fn ready_msg(&self) -> worker_to_gw_request::Request
    {
        worker_to_gw_request::Request::WorkerReady(
            protobuf::WorkerReady {
                version: self
                    .version
                    .clone(),
                worker_class: self
                    .class
                    .clone(),
            },
        )
    }

    pub async fn connect(
        &self
    ) -> Result<(
        mpsc::Receiver<WorkerToGwResponse>,
        mpsc::Sender<WorkerToGwRequest>,
    )>
    {
        // channel that goes from grpc to user logic
        let (to_logic_tx, to_logic_rx) = tokio::sync::mpsc::channel(1);
        // channel that goes from user logic to grpc
        let (from_logic_tx, mut from_logic_rx) = tokio::sync::mpsc::channel(1);
        let uri = self
            .gateway
            .parse::<tonic::transport::Uri>()?;
        info!("Connecting to Gateway at uri `{uri}`");
        let channel = tonic::transport::Channel::builder(uri)
            .connect()
            .await?;
        let token: MetadataValue<_> = format!(
            "Bearer {}",
            self.token
        )
        .parse()?;

        let max_message_size = self
            .max_grpc_message_size_mb
            .unwrap_or(MAX_GRPC_MESSAGE_SIZE_MB)
            * 1024
            * 1024;

        let (outbound, outbound_rx) = tokio::sync::mpsc::channel(1024);
        let outbound_rx = tokio_stream::wrappers::ReceiverStream::new(outbound_rx);
        let mut client = protobuf::workers_service_client::WorkersServiceClient::with_interceptor(
            channel,
            move |mut req: Request<()>| {
                req.metadata_mut()
                    .insert(
                        "authorization",
                        token.clone(),
                    );
                Ok(req)
            },
        )
        .max_decoding_message_size(max_message_size)
        .max_encoding_message_size(max_message_size);

        let response = client
            .worker_to_gw(tonic::Request::new(outbound_rx))
            .await?;

        let mut inbound = response.into_inner();

        info!("Signalling to the GW worker is ready to accept tasks");
        outbound
            .send(
                WorkerToGwRequest {
                    request: Some(self.ready_msg()),
                },
            )
            .await?;

        info!("Worker entering loop task");
        tokio::spawn(
            async move {
                loop
                {
                    tokio::select! {
                        Some(inbound_message) = inbound.next() => {
                            let msg = match inbound_message {
                                Ok(msg) => msg,
                                Err(e) => {
                                    error!("connection to the gateway ended with status: {e}");
                                    break;
                                }
                            };
                            to_logic_tx.send(msg).await.expect("unable to send to gateway task to user logic");
                            // right now we only support one off message: GW -> worker -> GW, there is no
                            // support for sending multiple tasks at once to the same worker. For this,
                            // batching at the task level is preferred.
                            let out = from_logic_rx.recv().await.expect("unable to read reply from user logic");
                            outbound.send(out).await.expect("unable to send back msg to GW on channel");
                        }
                        else => break,
                    }
                }
            },
        );

        Ok(
            (
                to_logic_rx,
                from_logic_tx,
            ),
        )
    }
}
