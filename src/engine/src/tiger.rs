//! Tiger Data (TimescaleDB) connection over TLS, shared by the T2 probe and T21's telemetry store.

/// Connects with TLS required, whatever the URL's `sslmode` says, and runs the connection on tokio.
pub async fn connect(url: &str) -> anyhow::Result<tokio_postgres::Client> {
    let mut config: tokio_postgres::Config = url.parse()?;
    config.ssl_mode(tokio_postgres::config::SslMode::Require); // never fall back to plain text
    // shortcut: encrypted but the server is not verified (libpq's sslmode=require); macOS cannot verify Tiger's
    // certificate (SPEC decisions, Tiger TLS). Upgrade to OpenSSL with the Timescale CA pinned if the threat model needs it.
    let tls = native_tls::TlsConnector::builder().danger_accept_invalid_certs(true).build()?;
    let connector = postgres_native_tls::MakeTlsConnector::new(tls);
    let (client, connection) = config.connect(connector).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("tiger connection ended with error: {e}");
        }
    });
    Ok(client)
}
