//! Span host process.
//!
//! `span doctor` checks this machine against the support floor.
//! `span self-test` runs both ends of the pairing handshake on localhost.
//! `span host` listens for a device.

mod doctor;
mod identity;
mod session;

use std::env;
use std::io;
use std::net::SocketAddr;
use std::process::ExitCode;
use std::sync::Arc;

use mdns_sd::{ServiceDaemon, ServiceInfo};
use rustls::pki_types::ServerName;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor, TlsConnector};

use identity::{default_data_dir, fingerprint, Identity};
use session::{device_handshake, host_handshake, offer};

fn main() -> ExitCode {
    match real_main() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("span: {error}");
            ExitCode::from(1)
        }
    }
}

fn real_main() -> io::Result<u8> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("doctor") => Ok(u8::try_from(doctor::run()?).unwrap_or(1)),
        Some("self-test") => {
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(tls_self_test())?;
            println!("self-test: TLS и рукопожатие прошли, ключи совпали");
            Ok(0)
        }
        Some("host") => {
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(serve(args))?;
            Ok(0)
        }
        Some("device") => {
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(run_device(args))?;
            Ok(0)
        }
        _ => {
            println!(
                "span doctor\nspan self-test\nspan host [--name ИМЯ] [--port ПОРТ] [--data-dir ПУТЬ]\nspan device --host АДРЕС:ПОРТ [--name ИМЯ]"
            );
            Ok(2)
        }
    }
}

async fn tls_self_test() -> io::Result<()> {
    let identity = Identity::generate()?;
    let pin = identity.fingerprint();
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let acceptor = TlsAcceptor::from(Arc::new(identity.server_config()?));
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await?;
        let tls = acceptor.accept(stream).await?;
        host_handshake(tls, "Span").await
    });
    let client = tokio::spawn(async move {
        let stream = TcpStream::connect(("127.0.0.1", port)).await?;
        let tls = TlsConnector::from(device_connector(&pin)?)
            .connect(
                ServerName::try_from("span-host.local").map_err(invalid)?,
                stream,
            )
            .await?;
        device_handshake(tls, "test-device", [9u8; 16]).await
    });
    let (host, device) = tokio::try_join!(join(server), join(client))?;
    if host.media_key != device.media_key || host.session_id != device.session_id {
        return Err(invalid("ключи разошлись"));
    }
    Ok(())
}

async fn join<T>(handle: tokio::task::JoinHandle<io::Result<T>>) -> io::Result<T> {
    match handle.await {
        Ok(result) => result,
        Err(error) => Err(invalid(error)),
    }
}

async fn serve(args: impl Iterator<Item = String>) -> io::Result<()> {
    let options = HostOptions::parse(args)?;
    let identity = Identity::load_or_create(&options.data_dir)?;
    let listener = TcpListener::bind(("0.0.0.0", options.port)).await?;
    let port = listener.local_addr()?.port();
    let text = offer(&identity, port, &options.name)?;
    println!("Span слушает 0.0.0.0:{port}");
    println!("Данные: {}", options.data_dir.display());
    println!("Отпечаток: {}", hex(&identity.fingerprint()));
    println!("Предложение для QR:\n{text}");
    if let Ok(code) = qrcode::QrCode::new(text.as_bytes()) {
        let image = code
            .render::<qrcode::render::unicode::Dense1x2>()
            .quiet_zone(true)
            .build();
        println!("{image}");
    }
    let _mdns = advertise(&identity, port, &options.name);
    let acceptor = TlsAcceptor::from(Arc::new(identity.server_config()?));
    loop {
        let (stream, peer) = listener.accept().await?;
        let acceptor = acceptor.clone();
        let name = options.name.clone();
        tokio::spawn(async move {
            if let Err(error) = accept_tls(acceptor, stream, peer, &name).await {
                eprintln!("сессия с {peer} не открылась: {error}");
            }
        });
    }
}

async fn accept_tls(
    acceptor: TlsAcceptor,
    stream: tokio::net::TcpStream,
    peer: SocketAddr,
    name: &str,
) -> io::Result<()> {
    let tls = acceptor.accept(stream).await?;
    let established = host_handshake(tls, name).await?;
    println!(
        "Сессия с {peer}: устройство «{}», id {}",
        established.device_name,
        hex(&established.session_id)
    );
    Ok(())
}

async fn run_device(args: impl Iterator<Item = String>) -> io::Result<()> {
    let mut host = None;
    let mut name = "Span device".to_string();
    let mut iter = args.peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--host" => host = iter.next(),
            "--name" => {
                name = iter
                    .next()
                    .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "--name"))?;
            }
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("неизвестный аргумент {other}"),
                ));
            }
        }
    }
    let host = host.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "--host"))?;
    let address: SocketAddr = host
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "адрес host"))?;
    let stream = tokio::net::TcpStream::connect(address).await?;
    let pinned = read_pinned_fingerprint()?;
    let connector = device_connector(&pinned)?;
    let tls = tokio_rustls::TlsConnector::from(connector)
        .connect(
            ServerName::try_from("span-host.local").map_err(invalid)?,
            stream,
        )
        .await?;
    let established = device_handshake(tls, &name, random_bytes()).await?;
    println!(
        "Подключено к {address}, сессия {}",
        hex(&established.session_id)
    );
    Ok(())
}

fn device_connector(pinned: &[u8; 32]) -> io::Result<Arc<rustls::ClientConfig>> {
    let provider = rustls::crypto::ring::default_provider();
    let verifier = Arc::new(PinVerifier {
        fingerprint: *pinned,
        provider: Arc::new(provider.clone()),
    });
    let config = rustls::ClientConfig::builder_with_provider(provider.into())
        .with_protocol_versions(&[&rustls::version::TLS12, &rustls::version::TLS13])
        .map_err(invalid)?
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth();
    Ok(Arc::new(config))
}

fn read_pinned_fingerprint() -> io::Result<[u8; 32]> {
    let text = env::var("SPAN_PIN").map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "SPAN_PIN должен содержать 64 шестнадцатеричных символа отпечатка",
        )
    })?;
    let bytes = hex_decode(text.trim()).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "SPAN_PIN не является отпечатком",
        )
    })?;
    bytes.try_into().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "отпечаток должен быть 32 байта",
        )
    })
}

struct HostOptions {
    name: String,
    port: u16,
    data_dir: std::path::PathBuf,
}

impl HostOptions {
    fn parse(args: impl Iterator<Item = String>) -> io::Result<Self> {
        let mut name = "Span".to_string();
        let mut port = 47000u16;
        let mut data_dir = default_data_dir();
        let mut iter = args.peekable();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--name" => {
                    name = iter
                        .next()
                        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "--name"))?;
                }
                "--port" => {
                    let text = iter
                        .next()
                        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "--port"))?;
                    port = text
                        .parse()
                        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "--port"))?;
                    if port == 0 {
                        return Err(io::Error::new(io::ErrorKind::InvalidInput, "--port"));
                    }
                }
                "--data-dir" => {
                    data_dir = iter
                        .next()
                        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "--data-dir"))?
                        .into();
                }
                other => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("неизвестный аргумент {other}"),
                    ));
                }
            }
        }
        Ok(Self {
            name,
            port,
            data_dir,
        })
    }
}

fn advertise(identity: &Identity, port: u16, name: &str) -> io::Result<ServiceDaemon> {
    let daemon = ServiceDaemon::new().map_err(invalid)?;
    let host = hostname();
    let ip = std::net::IpAddr::V4(primary_ipv4().unwrap_or(std::net::Ipv4Addr::LOCALHOST));
    let properties = [
        ("txtvers", "1"),
        ("proto", "1"),
        ("role", "host"),
        ("fp", leak(hex(&identity.fingerprint()))),
        ("id", leak(hex(&identity.host_id))),
        (
            "caps",
            leak(format!("{:016x}", session::host_capabilities())),
        ),
        ("name", leak(name.to_string())),
    ];
    let info = ServiceInfo::new(
        "_span._tcp.local.",
        &format!("{name}._span._tcp.local."),
        &format!("{host}.local."),
        ip,
        port,
        &properties[..],
    )
    .map_err(invalid)?;
    daemon.register(info).map_err(invalid)?;
    if ip.is_loopback() {
        eprintln!("mDNS объявлен только на 127.0.0.1: телефон в сети его не увидит");
    } else {
        println!("mDNS: _span._tcp на {ip}:{port}");
    }
    Ok(daemon)
}

fn primary_ipv4() -> Option<std::net::Ipv4Addr> {
    let output = std::process::Command::new("ip")
        .args(["-4", "-o", "addr", "show", "scope", "global"])
        .output()
        .ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    for line in text.lines() {
        let Some(address) = line.split_whitespace().find(|part| part.contains('/')) else {
            continue;
        };
        let ip = address.split('/').next()?;
        if let Ok(parsed) = ip.parse() {
            return Some(parsed);
        }
    }
    None
}

fn hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "span".to_string())
}

pub(crate) fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    getrandom::getrandom(&mut bytes).expect("операционная система не выдала случайные байты");
    bytes
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    out
}

fn hex_decode(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }
    let chars = text.as_bytes();
    let mut out = Vec::with_capacity(text.len() / 2);
    let mut index = 0;
    while index < chars.len() {
        let hi = unhex(chars[index])?;
        let lo = unhex(chars[index + 1])?;
        out.push((hi << 4) | lo);
        index += 2;
    }
    Some(out)
}

fn unhex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn invalid(error: impl std::fmt::Display) -> io::Error {
    io::Error::other(error.to_string())
}

fn leak(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

#[derive(Debug)]
struct PinVerifier {
    fingerprint: [u8; 32],
    provider: Arc<rustls::crypto::CryptoProvider>,
}

impl rustls::client::danger::ServerCertVerifier for PinVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        if fingerprint(end_entity.as_ref()) == self.fingerprint {
            Ok(rustls::client::danger::ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General(
                "отпечаток сертификата не совпал".into(),
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}
