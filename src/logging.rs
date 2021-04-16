use flexi_logger::{style, Level};

fn reduced_colored_format(
    w: &mut dyn std::io::Write,
    now: &mut flexi_logger::DeferredNow,
    record: &flexi_logger::Record,
) -> Result<(), std::io::Error> {
    let level = record.level();
    write!(
        w,
        "{} {:<5} {}:{} {} {}",
        now.now().format("%H:%M:%S"),
        style(level, level),
        record.file().unwrap_or("<unnamed>"),
        record.line().unwrap_or(0),
        style(level, ">"),
        &record.args(),
    )
}

fn fully_colored_format(
    w: &mut dyn std::io::Write,
    now: &mut flexi_logger::DeferredNow,
    record: &flexi_logger::Record,
) -> Result<(), std::io::Error> {
    let level = record.level();
    let part1 = format!("{} {:<5} {}:{} > ",
        now.now().format("%H:%M:%S"),
        level,
        record.file().unwrap_or("<unnamed>"),
        record.line().unwrap_or(0),
    );
    write!(w, "{}{}",
        style(level, part1),
        style(level, &record.args()),
    )
}

fn colored_format(
    w: &mut dyn std::io::Write,
    now: &mut flexi_logger::DeferredNow,
    record: &flexi_logger::Record,
) -> Result<(), std::io::Error> {
    let level = record.level();
    match level {
        Level::Error => fully_colored_format(w, now, record),
        Level::Warn => fully_colored_format(w, now, record),
        Level::Info => reduced_colored_format(w, now, record),
        Level::Debug => reduced_colored_format(w, now, record),
        Level::Trace => fully_colored_format(w, now, record),
    }

}

fn file_format(
    w: &mut dyn std::io::Write,
    now: &mut flexi_logger::DeferredNow,
    record: &flexi_logger::Record,
 ) -> Result<(), std::io::Error> {
    write!(
        w,
        "[{}] {:<5} [{}:{}] {}",
        now.now().format("%Y-%m-%d %H:%M:%S%.3f %:z"),
        record.level(),
        record.file().unwrap_or("<unnamed>"),
        record.line().unwrap_or(0),
        &record.args(),
    )
}

pub fn init_logging() {
    flexi_logger::Logger::with_env_or_str("trace")
        .print_message()
        .log_to_file()
        .format_for_files(file_format)
        .set_palette("196;208;120;141;241".to_string()) // https://jonasjacek.github.io/colors/
        .format_for_stderr(colored_format)
        .format_for_stdout(colored_format)
        .duplicate_to_stderr(flexi_logger::Duplicate::All)
        .directory("logs")
        .start().unwrap();

    // also log panics
    std::panic::set_hook(Box::new(|panic_info| {
        error!(target: "PANIC", "{}", panic_info);
    }));

    trace!("Awoo");
    debug!("Awoo");
    info!("Awoo");
    warn!("Awoo");
    error!("Awoo");
}
