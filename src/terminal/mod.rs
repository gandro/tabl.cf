use std::io::{self, Write};
use termcolor::{WriteColor, Ansi, NoColor, Color, ColorSpec};

use hyper::{Response, Body};
use self::html::Html;

mod html;

pub struct Terminal {
    w: TerminalImpl<Vec<u8>>
}

impl Terminal {
    pub fn ansi() -> Self {
        Terminal {
            w: TerminalImpl::Ansi(Ansi::new(Vec::new())),
        }
    }

    pub fn plain() -> Self {
        Terminal {
            w: TerminalImpl::Plain(NoColor::new(Vec::new())),
        }
    }

    pub fn html() -> Self {
        Terminal {
            w: TerminalImpl::Html(Html::new(Vec::new(), "<todo title>")
                .expect("writing to vec should not fail"))
        }
    }

    pub fn content_type(&self) -> &'static str {
        self.w.content_type()
    }
    
    pub fn body(self) -> Body {
        Body::from(self.w.into_inner())
    }
}

impl WriteColor for Terminal {
    fn supports_color(&self) -> bool {
        self.w.supports_color()
    }
    fn set_color(&mut self, spec: &ColorSpec) -> io::Result<()> {
        self.w.set_color(spec)
    }
    fn reset(&mut self) -> io::Result<()> {
        self.w.reset()
    }
}

impl Write for Terminal {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.w.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}

enum TerminalImpl<W> {
    Html(Html<W>),
    Ansi(Ansi<W>),
    Plain(NoColor<W>),
}

impl<W: Write> TerminalImpl<W> {
    fn into_inner(self) -> W {
        match self {
            TerminalImpl::Html(w) => w.into_inner(),
            TerminalImpl::Ansi(w) => w.into_inner(),
            TerminalImpl::Plain(w) => w.into_inner(),
        }
    }

    fn content_type(&self) -> &'static str {
        match self {
            TerminalImpl::Html(_) => "text/html; charset=UTF-8",
            TerminalImpl::Ansi(_) => "text/plain; charset=UTF-8",
            TerminalImpl::Plain(_) => "text/plain; charset=UTF-8",
        }
    }
}

impl<W: Write> WriteColor for TerminalImpl<W> {
    fn supports_color(&self) -> bool {
        match self {
            TerminalImpl::Html(w) => w.supports_color(),
            TerminalImpl::Ansi(w) => w.supports_color(),
            TerminalImpl::Plain(w) => w.supports_color(),
        }
    }
    fn set_color(&mut self, spec: &ColorSpec) -> io::Result<()> {
        match self {
            TerminalImpl::Html(w) => w.set_color(spec),
            TerminalImpl::Ansi(w) => w.set_color(spec),
            TerminalImpl::Plain(w) => w.set_color(spec),
        }
    }
    fn reset(&mut self) -> io::Result<()> {
        match self {
            TerminalImpl::Html(w) => w.reset(),
            TerminalImpl::Ansi(w) => w.reset(),
            TerminalImpl::Plain(w) => w.reset(),
        }
    }
}

impl<W: Write> Write for TerminalImpl<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            TerminalImpl::Html(w) => w.write(buf),
            TerminalImpl::Ansi(w) => w.write(buf),
            TerminalImpl::Plain(w) => w.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            TerminalImpl::Html(w) => w.flush(),
            TerminalImpl::Ansi(w) => w.flush(),
            TerminalImpl::Plain(w) => w.flush(),
        }
    }
}
