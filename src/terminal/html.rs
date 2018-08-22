use std::io::{self, Write};
use termcolor::{Ansi, Color, ColorSpec, NoColor, WriteColor};

pub struct Html<W> {
    writer: W,
    span_opened: bool,
}

impl<W: Write> Html<W> {
    pub fn new(mut writer: W, title: &str) -> io::Result<Self> {
        let mut html = Html {
            writer: writer,
            span_opened: false,
        };

        html.writer
            .write_all(b"<!doctype html><html lang=en><head><meta charset=utf-8><title>")?;
        html.write_html_encoded(title)?;
        html.writer.write_all(b"</title></head><body style=\"background-color: #000; color: #fff; font-family: monospace; white-space: pre;\">")?;

        Ok(html)
    }

    /// Consume this `Html` value and return the inner writer.
    pub fn into_inner(self) -> W {
        self.writer
    }

    /// Return a reference to the inner writer.
    pub fn get_ref(&self) -> &W {
        &self.writer
    }

    /// Return a mutable reference to the inner writer.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    fn write_html_encoded(&mut self, s: &str) -> io::Result<()> {
        for b in s.as_bytes() {
            match b {
                b'"' => self.writer.write_all(b"&quot;")?,
                b'&' => self.writer.write_all(b"&amp;")?,
                b'\'' => self.writer.write_all(b"&#x27;")?,
                b'<' => self.writer.write_all(b"&lt;")?,
                b'>' => self.writer.write_all(b"&gt;")?,
                byte => self.writer.write_all(&[*byte])?,
            }
        }
        Ok(())
    }

    fn write_css_color(&mut self, color: &Color) -> io::Result<()> {
        match color {
            Color::Black => self.writer.write_all(b"#000"),
            Color::Blue => self.writer.write_all(b"#00f"),
            Color::Green => self.writer.write_all(b"#0f0"),
            Color::Red => self.writer.write_all(b"#f00"),
            Color::Cyan => self.writer.write_all(b"#0ff"),
            Color::Magenta => self.writer.write_all(b"#f0f"),
            Color::Yellow => self.writer.write_all(b"#ff0"),
            Color::White => self.writer.write_all(b"#fff"),
            Color::Rgb(r, g, b) => write!(self.writer, "#{:02x}{:02x}{:02x}", r, g, b),
            _ => unimplemented!(),
        }
    }
}

impl<W: Write> Write for Html<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl<W: Write> WriteColor for Html<W> {
    fn supports_color(&self) -> bool {
        true
    }

    fn set_color(&mut self, spec: &ColorSpec) -> io::Result<()> {
        if self.span_opened {
            self.write_all(b"</span>")?;
        }

        self.write_all(b"<span style=\"")?;
        if let Some(color) = spec.fg() {
            self.write_all(b"color:")?;
            self.write_css_color(color)?;
            self.write_all(b";")?;
        }

        if let Some(color) = spec.bg() {
            self.write_all(b"background-color:")?;
            self.write_css_color(color)?;
            self.write_all(b";")?;
        }

        if spec.underline() {
            self.write_all(b"text-decoration: underline;")?;
        }

        if spec.bold() {
            self.write_all(b"font-weight: bold;")?;
        }
        self.write_all(b"\">")?;

        self.span_opened = true;
        Ok(())
    }
    fn reset(&mut self) -> io::Result<()> {
        if self.span_opened {
            self.write_all(b"</span>")?;
        }
        self.span_opened = false;
        Ok(())
    }
}
