use std::io::Write;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

pub fn write(stream: &mut StandardStream, content: &str) {
    stream.set_color(ColorSpec::new().set_fg(None)).unwrap();
    write!(stream, "{content}").unwrap();
}

pub fn writeln(stream: &mut StandardStream, content: &str) {
    stream.set_color(ColorSpec::new().set_fg(None)).unwrap();
    writeln!(stream, "{content}").unwrap();
}

pub fn write_color(stream: &mut StandardStream, content: &str, color: Option<Color>) {
    stream.set_color(ColorSpec::new().set_fg(color)).unwrap();
    write!(stream, "{content}").unwrap();
}

pub fn writeln_color(stream: &mut StandardStream, content: &str, color: Option<Color>) {
    stream.set_color(ColorSpec::new().set_fg(color)).unwrap();
    writeln!(stream, "{content}").unwrap();
}

pub fn write_spec(stream: &mut StandardStream, content: &str, spec: &ColorSpec) {
    stream.set_color(spec).unwrap();
    write!(stream, "{content}").unwrap();
}

pub fn writeln_spec(stream: &mut StandardStream, content: &str, spec: &ColorSpec) {
    stream.set_color(spec).unwrap();
    writeln!(stream, "{content}").unwrap();
}
