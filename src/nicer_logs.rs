use std::io::Write;

use tracing::{Level, Subscriber};
use tracing_subscriber::{self, Layer};
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::Context;


pub struct CustomLayer<W> {
    writer: W,
    writer_is_stderr: bool,
    writer_max_level: Level,
    stderr_max_level: Level,
    ansi: bool
}

impl<W> CustomLayer<W>
where
    W: for<'a> MakeWriter<'a> + Send + 'static,
{
    pub fn new(writer: W, writer_is_stderr: bool, writer_max_level: Level, stderr_max_level: Level, ansi: bool) -> Self {
        Self {
            writer,
            writer_is_stderr,
            writer_max_level,
            stderr_max_level,
            ansi,
        }
    }
}

impl<S, W> Layer<S> for CustomLayer<W>
where
    S: Subscriber + for<'a> tracing::Subscriber + Send + 'static,
    W: for<'a> MakeWriter<'a> + Send + 'static,
{
    fn on_event(&self, event: &tracing::Event, _: Context<S>) {
        if event.metadata().level() > &self.writer_max_level && event.metadata().level() <= &self.stderr_max_level {
            return;
        }

        struct FieldVisitor {
            pub message: String,
        }

        impl tracing::field::Visit for FieldVisitor {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    self.message = format!("{:?}", value);
                }
            }
        }

        let mut visitor = FieldVisitor {
            message: String::new(),
        };
        event.record(&mut visitor);

        let ev_level = event.metadata().level();
        let ev_file = event.metadata().file();
        let ev_line = event.metadata().line();
        let location = if ev_file.is_some() && ev_line.is_some() {
            format!(" {}:{}", ev_file.unwrap(), ev_line.unwrap())
        } else {
            "".to_string()
        };
        let now = chrono::Local::now();
        let timestamp = now.format("%H%M%S%.3f").to_string();
        // let my_msg = format!("{} {}{} {}", timestamp, ev_level, location, visitor.message);

        let mut already_have_in_stderr = false;
        if event.metadata().level() <= &self.stderr_max_level {
            if self.ansi {
                let _ = writeln!(std::io::stderr(), "{} \x1b[31m{}\x1b[0m{} {}", timestamp, ev_level, location, visitor.message);
            } else {
                let _ = writeln!(std::io::stderr(), "{} {}{} {}", timestamp, ev_level, location, visitor.message);
            }
            already_have_in_stderr = true;
        }

        if (!already_have_in_stderr || !self.writer_is_stderr) && event.metadata().level() <= &self.writer_max_level {
            let mut writer = self.writer.make_writer();
            let _ = writeln!(writer, "{} {}{} {}", timestamp, ev_level, location, visitor.message);
        }
    }
}


pub fn first_n_chars(msg: &String, n: usize) -> String {
    let mut last_n_chars: String = msg.chars().take(n).collect();
    if last_n_chars.len() == n {
        last_n_chars.push_str("...");
    }
    return last_n_chars.replace("\n", "\\n");
}

pub fn last_n_chars(msg: &String, n: usize) -> String {
    let mut last_n_chars: String = msg.chars().rev().take(n).collect::<String>().chars().rev().collect();
    if last_n_chars.len() == n {
        last_n_chars.insert_str(0, "...");
    }
    return last_n_chars.replace("\n", "\\n");
}
