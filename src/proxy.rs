use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use axum::body::Bytes;
use pingora_core::Error;
use pingora_core::prelude::{background_service, HttpPeer, Server};
use pingora_http::{RequestHeader, ResponseHeader};
use pingora_load_balancing::LoadBalancer;
use pingora_load_balancing::prelude::{RoundRobin, TcpHealthCheck};
use pingora_proxy::{ProxyHttp, PurgeStatus, Session};

struct LB(Arc<LoadBalancer<RoundRobin>>);


#[async_trait]
impl ProxyHttp for LB {
    type CTX = ();
    fn new_ctx(&self) -> Self::CTX {}
   async fn upstream_peer(&self, _session: &mut Session, _ctx: &mut Self::CTX) -> pingora_core::Result<Box<HttpPeer>> {
       let upstream = self.0.select(b"", 256).unwrap();
       log::info!("Forwarding request to {upstream:?}" );
       Ok(Box::from(HttpPeer::new(upstream, false, String::from(""))))
    }

    async
    fn request_filter(&self, _session: &mut Session, _ctx: &mut Self::CTX) -> pingora_core::Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        if !_session.req_header().uri.path().starts_with("/health") {
            let _ = _session.respond_error(403).await;
            return Ok(true)
        }
        Ok (false)
    }

    async
    fn upstream_request_filter(&self, _session: &mut Session, _upstream_request: &mut RequestHeader, _ctx: &mut Self::CTX) -> pingora_core::Result<()>
    where
        Self::CTX: Send + Sync,
    {
        _upstream_request.insert_header("x-proxy-from", "0.0.0.0:6193").unwrap();
        Ok(())
    }

    async
    fn response_filter(&self, _session: &mut Session, _upstream_response: &mut ResponseHeader, _ctx: &mut Self::CTX) -> pingora_core::Result<()>
    where
        Self::CTX: Send + Sync,
    {
        _upstream_response.insert_header("Name", "Muktadir").unwrap();
        Ok(())
    }

    async
    fn request_body_filter(&self, _session: &mut Session, _body: &mut Option<Bytes>, _end_of_stream: bool, _ctx: &mut Self::CTX) -> pingora_core::Result<()>
    where
        Self::CTX: Send + Sync,
    {
        todo!()
    }

    async fn response_body_filter(&self, _session: &mut Session, _body: &mut Option<Bytes>, _end_of_stream: bool, _ctx: &mut Self::CTX) -> pingora_core::Result<Option<Duration>>
    where
        Self::CTX: Send + Sync,
    {
        todo!()
    }

    async
    fn logging(&self, _session: &mut Session, _e: Option<&Error>, _ctx: &mut Self::CTX)
    where
        Self::CTX: Send + Sync,
    {
        todo!()
    }
}


fn main() {
    env_logger::init();

    let mut server = Server::new(None).expect("Failed to create server");
    server.bootstrap();

    log::info!("Server is bootstrapped");

    let mut upstreams = LoadBalancer::try_from_iter(["127.0.0.1:3000", "127.0.0.1:4000"]).unwrap();

    let hc = TcpHealthCheck::new();
    upstreams.set_health_check(hc);
    upstreams.health_check_frequency = Some(Duration::from_secs(10));

    let background = background_service("Health checker", upstreams);
    let upstreams = background.task();


    let mut proxy = pingora_proxy::http_proxy_service(&server.configuration, LB(upstreams));

    proxy.add_tcp("0.0.0.0:6193");
    log::info!("Proxy service added on 0.0.0.0:6193");

    server.add_service(proxy);
    server.run_forever();
}