use crate::supabase::SupabaseConfig;
use crate::SyncError;
use std::time::Duration;
use tungstenite::{connect, Message};

pub fn package_name_from_realtime(text: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let items = value.as_array()?;
    if items.get(3)?.as_str()? != "postgres_changes" {
        return None;
    }
    let payload = items.get(4)?;
    let name = payload
        .pointer("/data/new/package_name")
        .or_else(|| payload.pointer("/data/record/package_name"))
        .and_then(|item| item.as_str())?;
    (!name.is_empty()).then(|| name.to_string())
}

pub fn realtime_url(cfg: &SupabaseConfig) -> String {
    let host = cfg
        .url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    format!(
        "wss://{host}/realtime/v1/websocket?apikey={}&vsn=1.0.0",
        query_encode(&cfg.key)
    )
}

fn query_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn join_message(token: &str) -> String {
    serde_json::json!([
        "1",
        "1",
        "realtime:public:package_signals",
        "phx_join",
        {
            "config": {
                "broadcast": {"ack": false, "self": false},
                "presence": {"key": ""},
                "postgres_changes": [{
                    "event": "*",
                    "schema": "public",
                    "table": "package_signals"
                }],
                "private": false
            },
            "access_token": token
        }
    ])
    .to_string()
}

/// `still` torna false quando il fornitore cambia. Un errore di rete fa riconnettere il chiamante.
pub fn listen<F, G>(
    cfg: &SupabaseConfig,
    token: &str,
    mut on_package: F,
    still: G,
) -> Result<(), SyncError>
where
    F: FnMut(&str),
    G: Fn() -> bool,
{
    if !still() {
        return Ok(());
    }
    let (mut socket, _) = connect(realtime_url(cfg)).map_err(|e| SyncError::Http(e.to_string()))?;
    set_read_timeout(&socket);
    socket
        .send(Message::Text(join_message(token).into()))
        .map_err(|e| SyncError::Http(e.to_string()))?;
    let mut ticks = 0u32;
    loop {
        if !still() {
            return Ok(());
        }
        match socket.read() {
            Ok(Message::Text(text)) => {
                if let Some(package) = package_name_from_realtime(&text) {
                    on_package(&package);
                }
            }
            Ok(Message::Ping(data)) => {
                let _ = socket.send(Message::Pong(data));
            }
            Ok(Message::Close(_)) => return Err(SyncError::Http("realtime chiuso".into())),
            Ok(_) => {}
            Err(tungstenite::Error::Io(err))
                if err.kind() == std::io::ErrorKind::WouldBlock
                    || err.kind() == std::io::ErrorKind::TimedOut =>
            {
                ticks += 1;
                if ticks >= 5 {
                    ticks = 0;
                    socket
                        .send(Message::Text(
                            r#"["2","2","phoenix","heartbeat",{}]"#.into(),
                        ))
                        .map_err(|e| SyncError::Http(e.to_string()))?;
                }
            }
            Err(err) => return Err(SyncError::Http(err.to_string())),
        }
    }
}

fn set_read_timeout(
    socket: &tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
) {
    let tcp = match socket.get_ref() {
        tungstenite::stream::MaybeTlsStream::Plain(stream) => stream,
        tungstenite::stream::MaybeTlsStream::NativeTls(stream) => stream.get_ref(),
        _ => return,
    };
    let _ = tcp.set_read_timeout(Some(Duration::from_secs(5)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_package_from_a_postgres_change() {
        let text = r#"["1","2","realtime:public:package_signals","postgres_changes",{"data":{"new":{"package_name":"com.foo"}}}]"#;
        assert_eq!(package_name_from_realtime(text).as_deref(), Some("com.foo"));
        assert!(package_name_from_realtime(r#"["1","2","phoenix","heartbeat",{}]"#).is_none());
    }
}
