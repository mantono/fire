use std::io::Write;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

pub fn write(stream: &mut StandardStream, content: &str, color: Option<Color>) {
    stream.set_color(ColorSpec::new().set_fg(color)).unwrap();
    write!(stream, "{}", content).unwrap();
}
