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

use std::env;
use std::io;

use hyper::service::service_fn;
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};

//use hyper_tls::HttpsConnector;

use failure::Error;
use futures::future::{self, Future};

mod backend;
mod terminal;

use self::backend::*;
use self::terminal::*;

type ResponseFuture = Box<Future<Item = Response<Body>, Error = Error> + Send>;

struct Context<'a> {
    request: Request<Body>,
    backend: &'a Client,
}

impl<'a> Context<'a> {
    fn new(request: Request<Body>, backend: &'a Client) -> Context<'a> {
        Context { request, backend }
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.request
            .headers()
            .get(name)
            .map(|val| val.to_str())
            .and_then(|r| r.ok())
    }

    fn terminal(&self) -> Terminal {
        if let Some(mime) = self.header("Accept") {
            if mime.contains("text/html") {
                let title = self.header("Host").unwrap_or_default();
                return Terminal::html(title);
            }
        }

        if let Some(ua) = self.header("User-Agent") {
            if ua.starts_with("curl/") {
                return Terminal::ansi();
            }
        }

        Terminal::plain()
    }

    fn lookup(&self, station: &str) -> ResponseFuture {
        let mut terminal = self.terminal();
        let resp = self
            .backend
            .request(station)
            .submit()
            .and_then(move |station| {
                station.ansi_write(&mut terminal)?;
                let resp = Response::builder()
                    .header("Content-Type", terminal.content_type())
                    .body(Body::from(terminal.into_bytes()))
                    .expect("response builder should not fail");
                Ok(resp)
            });

        Box::new(resp)
    }

    fn search(&self, query: &str) -> ResponseFuture {
        unimplemented!()
    }

    fn usage(&self) -> ResponseFuture {
        let usage = future::ok(Response::new(Body::from("help page here")));
        Box::new(usage)
    }

    fn not_found(&self) -> ResponseFuture {
        let not_found = future::ok(
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Not Found"))
                .unwrap(),
        );

        Box::new(not_found)
    }

    fn route(&self) -> ResponseFuture {
        match (self.request.method(), self.request.uri().path()) {
            (&Method::GET, "/:help") | (&Method::GET, "/help") => self.usage(),
            (&Method::GET, "/favicon.ico") => self.not_found(),
            (&Method::GET, path) if path.starts_with("/~") => {
                let query = &path[2..];
                self.search(query)
            }
            (&Method::GET, path) if path.starts_with("/") => {
                let station = &path[1..];
                self.lookup(station)
            }
            _ => self.not_found(),
        }
    }

    fn dispatch(&self) -> impl Future<Item = Response<Body>, Error = hyper::Error> {
        self.route().or_else(|err| {
            error!("internal server error: {}", err);
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(err.to_string()))
                .unwrap())
        })
    }
}

fn main() -> Result<(), Error> {
    pretty_env_logger::init();

    let port = env::var("PORT")
        .ok()
        .map(|i| i.parse())
        .unwrap_or(Ok(8080))?;

    let addr = ([0, 0, 0, 0], port).into();

    let server = Server::bind(&addr)
        .serve(|| -> Result<_, io::Error> {
            let client = backend::Client::new()?;
            Ok(service_fn(move |req| Context::new(req, &client).dispatch()))
        }).map_err(|e| error!("server error: {:?}", e));

    println!("Listening on http://{}", addr);

    hyper::rt::run(server);
    Ok(())
}
