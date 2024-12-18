use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use pingora_core::prelude::{background_service, HttpPeer, Server};
use pingora_load_balancing::LoadBalancer;
use pingora_load_balancing::prelude::{RoundRobin, TcpHealthCheck};
use pingora_proxy::{ProxyHttp, Session};

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