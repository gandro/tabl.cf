extern crate bytes;
extern crate hyper;
extern crate hyper_tls;
extern crate pretty_env_logger;
#[macro_use]
extern crate log;

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate failure;
extern crate futures;

extern crate chrono;

extern crate num_cpus;
extern crate termcolor;

use std::io;

//use hyper::client::{Client, HttpConnector};
use hyper::rt;
use hyper::service::service_fn;
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};

//use hyper_tls::HttpsConnector;

use failure::Error;
use futures::future::{self, Future};
use futures::stream::Stream;

mod backend;
mod terminal;

use self::backend::*;
use self::terminal::*;

type ResponseFuture = Box<Future<Item = Response<Body>, Error = Error> + Send>;

fn lookup(client: &Client, station: &str) -> ResponseFuture {
    let mut terminal = Terminal::ansi();
    let resp = client.request(station).submit().and_then(move |station| {
        station.ansi_write(&mut terminal)?;
        let resp = Response::builder()
            .header("Content-Type", terminal.content_type())
            .body(Body::from(terminal.into_bytes()))
            .expect("response builder should not fail");
        Ok(resp)
    });

    Box::new(resp)
}

fn usage() -> ResponseFuture {
    let usage = future::ok(Response::new(Body::from("help page here")));

    Box::new(usage)
}

fn search(client: &Client, quer: &str) -> ResponseFuture {
    unimplemented!()
}

fn not_found() -> ResponseFuture {
    let not_found = future::ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not Found"))
        .unwrap());

    Box::new(not_found)
}

fn route(req: Request<Body>, client: &backend::Client) -> ResponseFuture {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/:help") | (&Method::GET, "/help") => {
            usage()
        }
        (&Method::GET, "/favicon.ico") => {
            not_found()
        }
        (&Method::GET, path) if path.starts_with("/~") => {
            let query = &path[2..];
            search(client, query)
        }
        (&Method::GET, path) if path.starts_with("/") => {
            let station = &path[1..];
            lookup(client, station)
        }
        _ => not_found(),
    }
}

fn report_error(err: Error) -> Result<Response<Body>, hyper::Error> {
    error!("internal server error: {}", err);
    Ok(Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::from(err.to_string()))
        .unwrap())
}

fn main() {
    pretty_env_logger::init();

    let addr = ([0, 0, 0, 0], 8080).into();

    let server = Server::bind(&addr)
        .serve(|| -> Result<_, io::Error> {
            let client = backend::Client::new()?;
            Ok(service_fn(move |req| {
                route(req, &client).or_else(report_error)
            }))
        }).map_err(|e| error!("server error: {:?}", e));

    println!("Listening on http://{}", addr);

    hyper::rt::run(server);
}
