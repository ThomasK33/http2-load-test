# HTTP/2 Load Testing Tool

## Description

This Rust-based command-line tool performs HTTP/2 load testing on a specified
server address. It uses Tokio for asynchronous operations and Hyper for making
HTTP/2 requests.

## Usage

### Command-line Arguments

- `ADDRESS`: Server address in the format `hostname:port`
- `--rate <RATE>`: Target request rate (requests per second) [default: 1]
- `--total <TOTAL>`: Total number of requests to execute [default: 1]

### Example

```sh
cargo run -- --rate=1000 --total=10000 localhost:8080
```

### Output

The following load-test was run against a local HTTP/2 server.
Specifically, the h2 example server located at [h2/examples/server.rs](https://github.com/hyperium/h2/blob/master/examples/server.rs).

To run the example server locally one should:

```sh
git clone https://github.com/hyperium/h2.git
cd h2
cargo run --example server
```

To then run a load-test, one can use:

```sh
❯ cargo run -- --total 100 --rate 50 localhost:5928
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s
     Running `target/debug/http2-load-test --total 100 --rate 50 'localhost:5928'`
success: 100.0%
median response time: 1.62ms
average in-flight: 0.10
```

Alternatively, one can also run a load-test against
an existing server, such as:

```sh
❯ cargo run -- --total 100 --rate 50 https://nghttp2.org/
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running `target/debug/http2-load-test --total 100 --rate 50 'https://nghttp2.org/'`
success: 100.0%
median response time: 511.93ms
average in-flight: 19.15
```

- `success`: The percentage of successful requests
- `median response time`: The median response time of the requests
- `average in-flight`: The average number of in-flight requests during the test

## Assumptions

- The tool focuses on GET requests for simplicity.
- Assumes a reasonable rate and total requests to avoid overwhelming the server.

## Design Decisions

- Used `hyper` directly for HTTP/2 support.
- Leveraged `tokio` for asynchronous operations.
- Chose `clap` for command-line argument parsing due to its ease of use and integration.

## TLS Support

Currently, this tool does not support TLS. However, adding TLS support can be a future
enhancement. One could use libraries such as `hyper-tls` or `hyper-rustls` to implement
TLS on top of the existing setup. These libraries provide convenient wrappers around
the Hyper client to enable secure connections.

## Future Improvements

- Add more detailed error handling.
- Extend the tool to support other HTTP methods.
- Add TLS support using libraries like `hyper-tls` or `hyper-rustls`.
