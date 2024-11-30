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
