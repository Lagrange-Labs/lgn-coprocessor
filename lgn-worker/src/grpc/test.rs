use std::net::ToSocketAddrs;
use std::pin::Pin;

use anyhow::Context;
use async_std::channel;
use tokio_stream::Stream;
use tokio_stream::StreamExt;
use tonic::metadata::MetadataMap;
use tonic::Response;

use super::protobuf::worker_to_gw_request;
use super::protobuf::worker_to_gw_response;
use super::protobuf::WorkerToGwRequest;
use super::protobuf::WorkerToGwResponse;
use super::protobuf::{
    self,
};

struct TestGateway
{
    // channel to send out msg from workers to testing logic
    msg_from_worker: async_std::channel::Sender<worker_to_gw_request::Request>,
    // channel to receive testing logic task to send to worker
    to_worker: async_std::channel::Receiver<worker_to_gw_response::Response>,
}

impl TestGateway
{
    pub async fn run(
        listen: &str
    ) -> anyhow::Result<(
        channel::Receiver<worker_to_gw_request::Request>,
        channel::Sender<worker_to_gw_response::Response>,
    )>
    {
        let (gw, from_worker, to_worker) = Self::new();

        let server = protobuf::workers_service_server::WorkersServiceServer::new(gw);
        let addr = listen.to_string();

        tokio::spawn(
            async move {
                if let Err(e) = tonic::transport::Server::builder()
                    .add_service(server)
                    .serve(
                        addr.to_socket_addrs()
                            .unwrap()
                            .next()
                            .unwrap(),
                    )
                    .await
                    .with_context(|| "what - error serving gw")
                {
                    panic!(
                        "error serving gw: {:?}",
                        e
                    );
                };
            },
        );
        Ok(
            (
                from_worker,
                to_worker,
            ),
        )
    }

    pub fn new() -> (
        Self,
        channel::Receiver<worker_to_gw_request::Request>,
        channel::Sender<worker_to_gw_response::Response>,
    )
    {
        let (from_worker_tx, from_worker_rx) = channel::bounded(10);
        let (to_worker_tx, to_worker_rx) = channel::bounded(10);
        (
            Self {
                msg_from_worker: from_worker_tx,
                to_worker: to_worker_rx,
            },
            from_worker_rx,
            to_worker_tx,
        )
    }
}

type GwTasksStream = Pin<
    Box<dyn Stream<Item = Result<protobuf::WorkerToGwResponse, tonic::Status>> + Send + 'static>,
>;

#[tonic::async_trait]
impl protobuf::workers_service_server::WorkersService for TestGateway
{
    type WorkerToGwStream = GwTasksStream;

    async fn worker_to_gw(
        &self,
        request: tonic::Request<tonic::Streaming<protobuf::WorkerToGwRequest>>,
    ) -> Result<tonic::Response<Self::WorkerToGwStream>, tonic::Status>
    {
        let mut stream = request.into_inner();

        let from_worker = self
            .msg_from_worker
            .clone();
        let to_worker = self
            .to_worker
            .clone();
        let output = async_stream::stream! {
            let first_message = stream
                .next()
                .await
                .ok_or(
                    tonic::Status::invalid_argument("A worker connection ended prematurely"),
                )?;

            let request = first_message?
                .request
                .ok_or(
                    tonic::Status::invalid_argument("the request field has to be populated"),
                )?;
            // signal to testing logic the worker has connected
            from_worker
                .send(request)
                .await.map_err(|_| tonic::Status::invalid_argument("can't send on outbound channel"))?;

            loop
            {
                tokio::select! {
                    message_from_worker = stream.next() => {
                        let Some(message_from_worker) = message_from_worker else {
                            break;
                        };

                        let message_from_worker = match message_from_worker {
                            Ok(m) => m,
                            Err(e) => {
                                yield Err(e);
                                break;
                            },
                        };
                        let request =  match message_from_worker.request.context("invalid WorkerToGwRequest msg") {
                            Ok(r) => r,
                            Err(e) => {
                                yield Err(tonic::Status::invalid_argument(format!("{:?}",e)));
                                break;
                            }
                        };
                        // signal message to worker to the testing logic
                        if let Err(e) = from_worker.send(request).await.map_err(|_| tonic::Status::invalid_argument("the request field has to be populated")) {
                            yield Err(e);
                            break;
                        };
                    }
                    Ok(message_to_worker) = to_worker.recv() => {
                        yield Ok(protobuf::WorkerToGwResponse { response: Some(message_to_worker) });
                    }
                }
            }
        };

        Ok(Response::new(Box::pin(output) as Self::WorkerToGwStream))
    }
}

#[cfg(test)]
mod grpc_test
{
    use anyhow::bail;

    use super::TestGateway;
    use crate::grpc::protobuf::worker_to_gw_response;
    use crate::grpc::protobuf::WorkerToGwResponse;
    use crate::grpc::GrpcConfig;

    #[tokio::test]
    async fn test_grpc_dummy_gateway() -> anyhow::Result<()>
    {
        let listen_address = "127.0.0.1:5678";
        let connect_uri = format!(
            "http://{}",
            listen_address
        );
        let (from_worker, to_worker) = TestGateway::run(listen_address).await?;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let worker_config = GrpcConfig {
            gateway: connect_uri.to_string(),
            version: "123".to_string(),
            token: "blou".to_string(),
            class: "super".to_string(),
            max_grpc_message_size_mb: None,
        };

        let (mut from_gw, to_gw) = worker_config
            .connect()
            .await?;
        // send first task
        let todo_sent = "this is big task".to_string();
        to_worker
            .send(worker_to_gw_response::Response::Todo(todo_sent.clone()))
            .await?;
        // expect to see task
        let recv_todo = async_std::future::timeout(
            std::time::Duration::from_millis(1000),
            from_gw.recv(),
        )
        .await?;
        let Some(WorkerToGwResponse {
            response: Some(worker_to_gw_response::Response::Todo(todo_received)),
        }) = recv_todo
        else
        {
            bail!("not normal");
        };
        assert_eq!(
            todo_sent,
            todo_received
        );
        Ok(())
    }
}
