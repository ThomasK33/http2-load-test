use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::anyhow;
use clap::Parser;
use futures::stream::{FuturesUnordered, StreamExt};
use http_body_util::Empty;
use hyper::body::Bytes;
use hyper::{Request, Uri};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpStream;
use tokio::time::sleep;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Server address in the format hostname:port
    address: String,

    /// Target request rate (requests per second)
    #[arg(short, long, default_value_t = 1.0)]
    rate: f64,

    /// Total number of requests to execute
    #[arg(short, long, default_value_t = 1)]
    total: usize,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    let uri = cli.address.parse::<hyper::Uri>()?;
    if uri.scheme().is_none() {
        return Err(anyhow!(
            "Missing URI schemes. Currently supported schemes are \"http://\"."
        ));
    }

    let delay = Duration::from_secs_f64(1.0 / cli.rate);
    let total_requests = cli.total;

    // Shared counters and vars
    let success_count: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    let response_times = Arc::new(Mutex::new(Vec::new()));

    // Get the host and the port
    let host = uri.host().expect("uri has no host");
    let port = uri.port_u16().unwrap_or(80);
    let address = format!("{}:{}", host, port);

    // Open a TCP connection to the remote host
    let stream = TcpStream::connect(address).await?;
    let io = TokioIo::new(stream);

    // Create the Hyper client
    let (sender, conn) = hyper::client::conn::http2::handshake(TokioExecutor::new(), io).await?;

    // Spawn a task to poll the connection, driving the HTTP state
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });

    // Perform the requests
    {
        let mut futures = FuturesUnordered::new();

        for _ in 0..total_requests {
            let mut sender = sender.clone();
            let uri = uri.clone();
            let success_count = success_count.clone();
            let response_times = response_times.clone();

            // The authority of our URL will be the hostname of the remote
            let authority = uri.authority().unwrap().clone();

            futures.push(tokio::spawn(async move {
                if let Ok(duration) = make_request(&mut sender, uri, authority.as_str()).await {
                    {
                        let mut sc = success_count.lock().unwrap();
                        *sc += 1;
                    }
                    let mut rt = response_times.lock().unwrap();
                    rt.push(duration);
                }
            }));

            sleep(delay).await;
        }

        while (futures.next().await).is_some() {}
    }

    // Gather and compute stats
    let success_count = *success_count.lock().unwrap();
    let response_times = response_times.lock().unwrap();

    let success_rate = (success_count as f64 / total_requests as f64) * 100.0;
    let median_response_time = {
        let mut times = response_times.clone();
        times.sort();
        if times.is_empty() {
            Duration::new(0, 0)
        } else {
            times[times.len() / 2]
        }
    };

    println!("success: {:.1}%", success_rate);
    println!("median response time: {:.2?}", median_response_time);

    Ok(())
}

async fn make_request(
    sender: &mut hyper::client::conn::http2::SendRequest<Empty<Bytes>>,
    uri: Uri,
    authority: &str,
) -> Result<Duration, anyhow::Error> {
    let start = Instant::now();

    // Create an HTTP request with an empty body and a HOST header
    let req = Request::builder()
        .uri(uri)
        .header(hyper::header::HOST, authority)
        .body(Empty::<Bytes>::new())?;

    // Await the response...
    sender.send_request(req).await?;

    Ok(start.elapsed())
}
