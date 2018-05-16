extern crate env_logger;
extern crate hyper;
extern crate hyper_tls;
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

extern crate termcolor;

use hyper::client::{Client, HttpConnector};
use hyper::rt;
use hyper::service::service_fn;
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};

use hyper_tls::HttpsConnector;

use failure::Error;
use futures::future::{self, Future};

mod backend;
mod terminal;

use self::backend::*;
use self::terminal::*;

type HttpsClient = Client<HttpsConnector<HttpConnector>>;

/*fn fetch(client: &HttpsClient, station: &str) -> impl Future<Item=Body, Error=Error> {

    println!("{:?}", station);
    let url = format!(
        "https://timetable.search.ch/api/stationboard.json?stop={}&show_delays=1",
        station
    );
    let url = url.parse::<hyper::Uri>().unwrap();
    client.get(url).map_err(Error::from)
        .and_then(|res| {
            res.into_body().concat2().map_err(Error::from).and_then(|body| {
                let mut buf = Vec::new();
                let s: Stationboard = serde_json::from_slice(&body)?;              
                s.ansi_write(&mut buf)?;
                Ok(Body::from(buf))
            })
    })
}*/

fn respond<T>(
    status: StatusCode,
    payload: T,
) -> Box<Future<Item = Response<Body>, Error = Error> + Send>
where
    T: Into<Body>,
{
    let response = Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(payload.into())
        .unwrap();
    Box::new(future::ok(response))
}

fn router(
    req: Request<Body>,
    client: &HttpsClient,
) -> impl Future<Item = Response<Body>, Error = Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/:help") | (&Method::GET, "/help") => {
            respond(StatusCode::OK, "help page here")
        }
        (&Method::GET, path) if path.starts_with("/~") => {
            respond(StatusCode::OK, format!("searching for: {}", &path[2..]))
        }
        (&Method::GET, path) if path.starts_with("/") => {
            let mut term = match req.headers().get(hyper::header::ACCEPT) {
                Some(accepted) if accepted.to_str().unwrap().contains("text/html") => Terminal::html(),
                Some(_) => Terminal::ansi(),
                None => Terminal::plain(),
            };
        
            let station = &path[1..];
            Box::new(
                backend::Request::new(client, station)
                    .submit()
                    //.map(|s| Response::new(Body::from(format!("{:#?}", s)))),
                    .and_then(move |station| {
                        station.ansi_write(&mut term)?;
                        let resp = Response::builder()
                            .header("Content-Type", term.content_type())
                            .body(term.body())
                            .expect("response builder should not fail");
                        Ok(resp)
                    })
            )
        }
        _ => respond(StatusCode::NOT_FOUND, "Not Found"),
    }
}

fn main() {
    env_logger::init();

    let addr = ([127, 0, 0, 1], 1337).into();

    let https = HttpsConnector::new(4).unwrap();
    let client = Client::builder().build::<_, hyper::Body>(https);

    let new_service = move || {
        let client = client.clone();
        service_fn(move |req| {
            router(req, &client).or_else(|err| {
                error!("internal server error: {}", err);
                Ok::<_, !>(
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from(err.to_string()))
                        .unwrap(),
                )
            })
        })
    };

    let server = Server::bind(&addr)
        .serve(new_service)
        .map_err(|e| error!("server error: {:?}", e));

    println!("Listening on http://{}", addr);

    hyper::rt::run(server);
}
