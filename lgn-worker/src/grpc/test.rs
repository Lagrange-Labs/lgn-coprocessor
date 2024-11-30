use std::pin::Pin;

use tokio_stream::Stream;

use super::protobuf;

struct TestGateway<W>
{
    validTokens: Vec<String>,
    workerHandler: W,
}

trait WorkerHandler
{
    fn new_worker(
        token: String,
        class: String,
    ) -> Option<protobuf::WorkerToGwRequest>;
    fn reply(
        token: String,
        reply: protobuf::WorkerToGwResponse,
    ) -> Option<protobuf::WorkerToGwRequest>;
}

#[tonic::async_trait]
impl protobuf::workers_service_server::WorkersService for TestGateway
{
    type WorkerToGwStream = Pin<
        Box<
            dyn Stream<Item = Result<protobuf::WorkerToGwResponse, tonic::Status>> + Send + 'static,
        >,
    >;

    async fn worker_to_gw(
        &self,
        request: tonic::Request<tonic::Streaming<lagrange::WorkerToGwRequest>>,
    ) -> Result<tonic::Response<Self::WorkerToGwStream>, tonic::Status>
    {
        // have to pass a reference to metadata because whole request
        // structure is not `Send` because of Streaming is not `Sync`
        let worker_details = self
            .auth_jwt(request.metadata())
            .await?;

        let worker_addr = request
            .remote_addr()
            .ok_or(tonic::Status::unimplemented("workers can only connect via TCP"))?;

        let mut stream = request.into_inner();

        let mut tx_dispatcher = self
            .tx_dispatcher
            .clone();

        // cannot use `try_stream!` because it cannot eat `?` inside `select!`:
        // <https://github.com/tokio-rs/async-stream/issues/63>
        let output = async_stream::stream!(
            {
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

                let mut channel = process_ready_message(
                    request,
                    &worker_addr,
                    worker_details.operator_id,
                    worker_details
                        .operator_name
                        .clone(),
                    &mut tx_dispatcher,
                )
                .await?;

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

                            if let Err(e) = process_message(
                                message_from_worker,
                                &mut tx_dispatcher,
                                worker_addr,
                            ).await { yield Err(e) };
                        }
                        message_to_worker = channel.recv() => {
                            if let Some(WorkerMessage::StartJob(envelope)) = message_to_worker {
                                let response = lagrange::worker_to_gw_response::Response::Todo(
                                    serde_json::to_string(&envelope).expect(
                                        "Can always serialise proof to json; qed"
                                    )
                                );

                                yield Ok(lagrange::WorkerToGwResponse { response: Some(response) });
                            }
                        }
                    }
                }
            }
        );

        Ok(Response::new(Box::pin(output) as Self::WorkerToGwStream))
    }
}
