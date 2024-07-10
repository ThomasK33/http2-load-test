use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use clap::Parser;
use futures::stream::{FuturesUnordered, StreamExt};
use http_body_util::Empty;
use hyper::body::Bytes;
use hyper::Uri;
use hyper_rustls::ConfigBuilderExt;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
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

    let mut uri = cli.address.parse::<hyper::Uri>()?;
    if uri.scheme().is_none() {
        uri = Uri::builder()
            .scheme("http")
            .authority(uri.authority().unwrap().as_str())
            .path_and_query(uri.path_and_query().map(|pq| pq.as_str()).unwrap_or(""))
            .build()
            .unwrap();
    }
    let uri = uri;

    let delay = Duration::from_secs_f64(1.0 / cli.rate);
    let total_requests = cli.total;

    // Shared counters and vars
    let success_count: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    let response_times = Arc::new(Mutex::new(Vec::new()));
    let in_flight = Arc::new(AtomicUsize::new(0));
    let in_flight_samples = Arc::new(Mutex::new(Vec::new()));

    // Create the Hyper client
    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(
            rustls::ClientConfig::builder()
                .with_native_roots()?
                .with_no_client_auth(),
        )
        .https_or_http()
        .enable_http2()
        .build();

    let client: Client<_, Empty<Bytes>> = Client::builder(TokioExecutor::new()).build(https);

    let in_flight_clone = in_flight.clone();
    let in_flight_samples_clone = in_flight_samples.clone();

    tokio::spawn(async move {
        loop {
            sleep(Duration::from_millis(100)).await;
            let sample = in_flight_clone.load(std::sync::atomic::Ordering::SeqCst);
            in_flight_samples_clone.lock().unwrap().push(sample);
        }
    });

    // Perform the requests
    {
        let mut futures = FuturesUnordered::new();

        for _ in 0..total_requests {
            let client = client.clone();
            let uri = uri.clone();
            let success_count = success_count.clone();
            let response_times = response_times.clone();
            let in_flight = in_flight.clone();

            futures.push(tokio::spawn(async move {
                if let Ok(duration) = make_request(client, uri, in_flight).await {
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
    let in_flight_samples = in_flight_samples.lock().unwrap();

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
    let average_in_flight = {
        let total_samples: usize = in_flight_samples.iter().sum();
        if in_flight_samples.len() > 0 {
            total_samples as f64 / in_flight_samples.len() as f64
        } else {
            0.0
        }
    };

    println!("success: {:.1}%", success_rate);
    println!("median response time: {:.2?}", median_response_time);
    println!("average in-flight: {:.2}", average_in_flight);

    Ok(())
}

async fn make_request(
    client: Client<
        hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
        Empty<Bytes>,
    >,
    uri: hyper::Uri,
    in_flight: Arc<AtomicUsize>,
) -> Result<Duration, anyhow::Error> {
    let start = Instant::now();

    // Await the response...
    in_flight.fetch_add(1, Ordering::SeqCst);
    client.get(uri).await?;
    in_flight.fetch_sub(1, Ordering::SeqCst);

    Ok(start.elapsed())
}
