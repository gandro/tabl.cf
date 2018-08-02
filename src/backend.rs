
use std::fmt;
use std::io::{self, Write};

use failure::Error;

use chrono::{DateTime, Local};
use serde::de::{self, Deserialize, Deserializer, Visitor};

use hyper;
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;

use failure;
use futures::future::{self, Either, Future};
use futures::stream::Stream;
use serde_json;
use num_cpus;

use termcolor::*;

type HttpsClient = hyper::Client<HttpsConnector<HttpConnector>>;

#[derive(Clone, Debug)]
pub struct Client {
    client: HttpsClient,
}

impl Client {
    pub fn new() -> io::Result<Self> {
        let cpus = num_cpus::get();
        let connector = HttpsConnector::new(cpus).
            map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let client = hyper::Client::builder()
            .build::<_, hyper::Body>(connector);

        Ok(Client {
            client: client
        })
    }

    pub fn request<'a, 'b>(&'a self, station: &'b str) -> Request<'a, 'b> {
        Request {
            client: &self.client,
            station: station,
            limit: 10,
            date: None,
            time: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Request<'a, 'b> {
    client: &'a HttpsClient,
    station: &'b str,
    limit: u32,
    date: Option<&'a str>,
    time: Option<&'a str>,
}

impl<'a, 'b> Request<'a, 'b> {
    pub fn submit(&self) -> impl Future<Item = Stationboard, Error = Error> {
        let url = format!(
            "https://timetable.search.ch/api/stationboard.json?stop={}&show_delays=1",
            self.station
        );
        println!("{}", url);
        let url = match url.parse::<hyper::Uri>() {
            Ok(url) => url,
            Err(err) => {
                let failure = Error::from(err);
                return Either::B(future::err(failure));
            }
        };

        let request = self.client.get(url).map_err(Error::from).and_then(|res| {
            res.into_body()
                .concat2()
                .map_err(Error::from)
                .and_then(|body| Response::decode(&body))
        });

        Either::A(request)
    }
}

#[derive(Debug, Serialize)]
pub struct Coord(pub u32);

#[derive(Serialize, Deserialize, Debug)]
pub struct Station {
    pub id: String,
    pub name: String,
    pub x: Coord,
    pub y: Coord,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Connection {
    #[serde(with = "local_datetime")]
    pub time: DateTime<Local>,
    #[serde(rename = "*L", default)]
    pub line_ty: String,
    #[serde(rename = "*G", default)]
    pub line_nr: String,
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(rename = "type_name")]
    pub ty_name: String,
    pub line: String,
    pub operator: String,
    pub color: String, // TODO
    pub number: String,
    pub terminal: Station,
    pub dep_delay: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    stop: Option<Station>,
    connections: Option<Vec<Connection>>,
    #[serde(default)]
    messages: Vec<String>,
    request: String,
    eof: u8,
}

impl Response {
    fn decode(bytes: &[u8]) -> Result<Stationboard, Error> {
        match serde_json::from_slice(&bytes)? {
            Response {
                stop: Some(stop),
                connections: Some(connections),
                ..
            } => Ok(Stationboard { stop, connections }),
            Response {
                ref mut messages, ..
            } if !messages.is_empty() =>
            {
                Err(failure::err_msg(messages.pop().unwrap()))
            }
            _ => bail!("malformed response from backend"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Stationboard {
    pub stop: Station,
    pub connections: Vec<Connection>,
}

pub struct Rgb(pub u8, pub u8, pub u8);

pub struct Colored {
    pub bg: Option<Rgb>,
    pub fg: Option<Rgb>,
    pub border: bool,
}

fn parse_ansi_color(c: &str) -> Option<Color> {
    let (r, g, b) = if c.len() == 3 {
        (&c[0..1], &c[1..2], &c[2..3])
    } else if c.len() == 6 {
        (&c[0..2], &c[2..4], &c[4..6])
    } else {
        return None;
    };

    let mut r = u8::from_str_radix(r, 16).ok()?;
    let mut g = u8::from_str_radix(g, 16).ok()?;
    let mut b = u8::from_str_radix(b, 16).ok()?;

    if c.len() == 3 {
        r = r << 4 | r;
        g = g << 4 | g;
        b = b << 4 | b;
    }

    Some(Color::Rgb(r, g, b))
}

impl Stationboard {
    pub fn ansi_write<W: WriteColor>(&self, mut w: W) -> io::Result<()> {
        w.set_color(ColorSpec::new().set_fg(Some(Color::White)).set_bold(true))?;
        writeln!(&mut w, "Timetable for {}", &*self.stop.name)?;

        for c in &self.connections {
            let mut colors = c.color.split("~");
            let bg = colors.next().and_then(parse_ansi_color);
            let fg = colors.next().and_then(parse_ansi_color);
            w.set_color(ColorSpec::new().set_bold(true).set_fg(fg).set_bg(bg))?;
            write!(&mut w, "{: ^3}", &c.line)?;
            w.reset()?;
            write!(&mut w, " {: <30}", &c.terminal.name)?;
            write!(&mut w, " {}", c.time.format("%H:%M"))?;
            if let Some(delay) = c.dep_delay.as_ref() {
                write!(&mut w, " {}", delay)?;
            }

            w.reset()?;
            writeln!(&mut w)?;
        }

        Ok(())
    }
}

mod local_datetime {
    use chrono::{DateTime, Local, TimeZone};
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";

    pub fn serialize<S>(date: &DateTime<Local>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Local>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Local
            .datetime_from_str(&s, FORMAT)
            .map_err(serde::de::Error::custom)
    }
}

struct CoordVisitor;

impl<'de> Visitor<'de> for CoordVisitor {
    type Value = Coord;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an integer between 0 and 1000000")
    }

    fn visit_i64<E>(self, value: i64) -> Result<Coord, E>
    where
        E: de::Error,
    {
        if value > 0 && value <= u32::max_value().into() {
            self.visit_u32(value as u32)
        } else {
            Err(de::Error::invalid_value(
                de::Unexpected::Signed(value),
                &self,
            ))
        }
    }

    fn visit_u64<E>(self, value: u64) -> Result<Coord, E>
    where
        E: de::Error,
    {
        if value <= u32::max_value().into() {
            self.visit_u32(value as u32)
        } else {
            Err(de::Error::invalid_value(
                de::Unexpected::Unsigned(value),
                &self,
            ))
        }
    }

    fn visit_u32<E>(self, value: u32) -> Result<Coord, E>
    where
        E: de::Error,
    {
        if value < 1_000_000 {
            Ok(Coord(value))
        } else {
            Err(de::Error::invalid_value(
                de::Unexpected::Unsigned(value as u64),
                &self,
            ))
        }
    }

    fn visit_str<E>(self, value: &str) -> Result<Coord, E>
    where
        E: de::Error,
    {
        value.parse().map(Coord).map_err(de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for Coord {
    fn deserialize<D>(deserializer: D) -> Result<Coord, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(CoordVisitor)
    }
}
