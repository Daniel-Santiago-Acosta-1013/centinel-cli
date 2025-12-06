use anyhow::Result;
use hickory_proto::op::{Message, Query};
use hickory_proto::rr::RecordType;
use hickory_resolver::TokioAsyncResolver;
use hickory_resolver::config::*;
use log::{info, warn};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::{Arc, mpsc};
use tokio::net::UdpSocket;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

pub struct DnsServerHandle {
    stop_tx: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

pub struct DnsServer;

impl DnsServer {
    pub fn start(listen: &str, blocklist: Vec<String>) -> Result<DnsServerHandle> {
        let addr: SocketAddr = listen.parse()?;
        let (stop_tx, stop_rx) = oneshot::channel::<()>();
        let (err_tx, err_rx) = mpsc::channel::<anyhow::Error>();
        let handle = std::thread::spawn(move || {
            let rt = Runtime::new().expect("runtime dns");
            let block = Arc::new(build_set(blocklist));
            let resolver =
                TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());
            rt.block_on(async move {
                if let Err(e) = serve(addr, resolver, block, stop_rx).await {
                    let _ = err_tx.send(e);
                }
            });
        });

        // errores inmediatos
        if let Ok(err) = err_rx.try_recv() {
            return Err(err);
        }

        Ok(DnsServerHandle {
            stop_tx: Some(stop_tx),
            thread: Some(handle),
        })
    }
}

impl DnsServerHandle {
    pub fn stop(&mut self) -> Result<()> {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
        Ok(())
    }
}

async fn serve(
    addr: SocketAddr,
    resolver: TokioAsyncResolver,
    blocklist: Arc<HashSet<String>>,
    mut stop: oneshot::Receiver<()>,
) -> Result<()> {
    let socket = UdpSocket::bind(addr).await?;
    info!("DNS local escuchando en {}", addr);
    loop {
        tokio::select! {
            _ = &mut stop => {
                info!("DNS detenido");
                break;
            }
            res = handle_once(&socket, &resolver, &blocklist) => {
                if let Err(e) = res {
                    warn!("DNS error: {e}");
                }
            }
        }
    }
    Ok(())
}

async fn handle_once(
    socket: &UdpSocket,
    resolver: &TokioAsyncResolver,
    blocklist: &HashSet<String>,
) -> Result<()> {
    let mut buf = [0u8; 512];
    let (len, peer) = socket.recv_from(&mut buf).await?;
    let req = Message::from_vec(&buf[..len])?;

    let mut resp = Message::new();
    resp.set_id(req.id());
    resp.set_message_type(req.message_type());
    resp.set_op_code(req.op_code());
    resp.set_recursion_desired(true);
    resp.set_recursion_available(true);

    if let Some(q) = req.queries().first() {
        if is_blocked(q, blocklist) {
            resp.set_response_code(hickory_proto::op::ResponseCode::NXDomain);
        } else {
            let name = q.name().to_ascii();
            let lookup = resolver.lookup_ip(name.clone()).await?;
            for ip in lookup.iter() {
                match ip {
                    std::net::IpAddr::V4(v4) => {
                        let record = hickory_proto::rr::Record::from_rdata(
                            q.name().clone(),
                            60,
                            hickory_proto::rr::RData::A(hickory_proto::rr::rdata::A(v4)),
                        );
                        resp.add_answer(record);
                    }
                    std::net::IpAddr::V6(v6) => {
                        let record = hickory_proto::rr::Record::from_rdata(
                            q.name().clone(),
                            60,
                            hickory_proto::rr::RData::AAAA(hickory_proto::rr::rdata::AAAA(v6)),
                        );
                        resp.add_answer(record);
                    }
                }
            }
        }
        resp.add_query(q.clone());
    }

    let out = resp.to_vec()?;
    socket.send_to(&out, &peer).await?;
    Ok(())
}

fn is_blocked(q: &Query, blocklist: &HashSet<String>) -> bool {
    if q.query_type() != RecordType::A && q.query_type() != RecordType::AAAA {
        return false;
    }
    let name = q.name().to_ascii().trim_end_matches('.').to_lowercase();
    blocklist.contains(&name)
}

fn build_set(list: Vec<String>) -> HashSet<String> {
    list.into_iter().collect()
}
