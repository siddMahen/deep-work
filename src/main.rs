use std::fs::{OpenOptions, remove_file};
use std::error::Error;
use std::path::Path;
use std::fmt::Display;
use std::env;

use ansi_term::Colour;
use chrono::prelude::*;
use chrono::TimeZone;
use clap::{Arg, App, SubCommand};
use csv::{ReaderBuilder, Writer, StringRecord};

static TIME_FMT: &str = "%H:%M:%S";
static DATE_FMT: &str = "%A, %B %e, %Y";
static DW_LOG: &str = ".dw.csv";
static DW_TMP: &str = ".dw.tmp";

static TXT_COLOUR: u8 = 13;

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("Deep Work Tracker")
            .version("0.1.0")
            .author("Siddharth Mahendraker <siddharth.mahen@gmail.com>")
            .about("A simple deep work time management tool")
            .subcommand(SubCommand::with_name("start")
                .about("Start tracking a deep work session")
                .arg(Arg::with_name("description")
                    .required(false)
                    .takes_value(true)
                    .short("d")
                    .long("desc")
                    .default_value("")
                    .hide_default_value(true)
                    .help("Description attached to this deep work session"))
                .arg(Arg::with_name("tags")
                    .required(false)
                    .multiple(true)
                    .takes_value(true)
                    .short("t")
                    .long("tag")
                    .default_value("")
                    .visible_alias("tags")
                    .hide_default_value(true)
                    .help("Tag(s) attached to this deep work session")))
            .subcommand(SubCommand::with_name("stop")
                .about("Stop tracking the current deep work session"))
            .subcommand(SubCommand::with_name("status")
                .about("Get the status of the current deep work session"))
            .subcommand(SubCommand::with_name("summary")
                .about("Summarize today's deep work"))
            .get_matches();

    let home = env::var("HOME")
        .expect("Failed to access HOME environment variable");
    let log_path = Path::new(&home).join(DW_LOG);
    let tmp_path = Path::new(&home).join(DW_TMP);

    let log_path_str = log_path.to_str()
        .expect("Failed to convert log path to string");
    let tmp_path_str = tmp_path.to_str()
        .expect("Failed to convert tmp path to string");

    if let Some(start) = matches.subcommand_matches("start") {
        let desc = start.value_of("description").unwrap();
        let tags: Vec<_> = start.values_of("tags").unwrap().collect();
        handle_start(tmp_path_str, desc, tags)?;
    } else if let Some(_) = matches.subcommand_matches("stop") {
        handle_stop(log_path_str, tmp_path_str)?;
    } else if let Some(_) = matches.subcommand_matches("status") {
        handle_status(tmp_path_str)?;
    } else if let Some(_) = matches.subcommand_matches("summary") {
        handle_summary(log_path_str)?;
    }

    Ok(())
}

fn handle_summary(log_path: &str) -> Result<(), Box<dyn Error>> {
    let file = OpenOptions::new()
        .read(true)
        .open(log_path)?;

    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .from_reader(file);

    let iter = reader.records();

    let mut total_dw_time = 0;

    for sess in iter {
        let record = sess.unwrap();
        let start = DateTime::parse_from_rfc3339(&record[0])?;
        let duration : i32 = (&record[2]).parse().unwrap();
        if start.date() == Local::now().date() {
            total_dw_time += duration;
        }
    }

    let hrs = total_dw_time/3600;
    let minutes = (total_dw_time/60) - 60*hrs;
    let seconds = total_dw_time - 60*minutes - 3600*hrs;

    let now = Local::now();

    println!("Deep work summary for {}:", now.format(DATE_FMT).to_string());
    println!("{} hour(s) {} minute(s) {} seconds(s)",
        Colour::Fixed(TXT_COLOUR).paint(hrs.to_string()),
        Colour::Fixed(TXT_COLOUR).paint(minutes.to_string()),
        Colour::Fixed(TXT_COLOUR).paint(seconds.to_string()));

    Ok(())
}

fn handle_start(tmp_path: &str, desc: &str, tags: Vec<&str>) -> Result<(), Box<dyn Error>> {
    let path = Path::new(tmp_path);

    if path.is_file() {
        println!("Another deep work session is active");
        return Ok(());
    }

    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(tmp_path)?;

    let mut writer = Writer::from_writer(file);
    let start = Local::now();

    writer.write_record(&[start.to_rfc3339(), desc.to_string(), tags.join(" ")])?;
    writer.flush()?;

    println!("Begin deep work!");
    print_start_time(start);
    print_description(desc);

    Ok(())
}

fn datetime_from_last_entry(path: &str) -> StringRecord {
    let file = OpenOptions::new()
        .read(true)
        .open(path)
        .expect("Failed to read temporary file");

    let mut reader = ReaderBuilder::new().
            has_headers(false).
            from_reader(file);

    let iter = reader.records();
    return iter.last().unwrap().unwrap();
}

fn handle_stop(log_path: &str, tmp_path: &str) -> Result<(), Box<dyn Error>> {
    let path = Path::new(tmp_path);

    if !path.is_file() {
        println!("No active deep work session");
        return Ok(());
    }

    let stop = Local::now();
    let record = datetime_from_last_entry(tmp_path);

    let start = DateTime::parse_from_rfc3339(&record[0])?;
    let desc  = &record[1];
    let tags = &record[2];

    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(log_path)?;

    let mut writer = Writer::from_writer(file);
    let elapsed = stop.signed_duration_since(start);

    writer.write_record(&[start.to_rfc3339(),
        stop.to_rfc3339(),
        elapsed.num_seconds().to_string(),
        desc.to_string(),
        tags.to_string()])?;
    writer.flush()?;

    println!("Deep work complete!");
    print_start_time(start);
    print_stop_time(stop);
    print_elapsed_time(start, stop);
    print_description(desc);
    print_tags(tags);

    remove_file(tmp_path)?;

    Ok(())
}

fn print_start_time<T: TimeZone>(time: DateTime<T>) where
    T::Offset: Display
{
    println!("Start: {}",
        Colour::Fixed(TXT_COLOUR).paint(time.format(TIME_FMT).to_string()));
}

fn print_stop_time<T: TimeZone>(time: DateTime<T>) where
    T::Offset: Display
{
    println!("Stop: {}",
        Colour::Fixed(TXT_COLOUR).paint(time.format(TIME_FMT).to_string()));
}

fn print_elapsed_time<S: TimeZone, T: TimeZone>(start: DateTime<S>, stop: DateTime<T>) {
    let elapsed = stop.signed_duration_since(start);
    let hrs = elapsed.num_hours();
    let min = elapsed.num_minutes() - 60*hrs;
    let sec = elapsed.num_seconds() - 3600*hrs - 60*min;
    println!("Time Elapsed: {} hour(s), {} minute(s), {} second(s)",
        Colour::Fixed(TXT_COLOUR).paint(hrs.to_string()),
        Colour::Fixed(TXT_COLOUR).paint(min.to_string()),
        Colour::Fixed(TXT_COLOUR).paint(sec.to_string()));
}

fn print_description(desc: &str) {
    if desc.len() > 0 {
        println!("Description: {}", desc);
    }
}

fn print_tags(tags: &str) {
    if tags.len() > 0 {
        println!("Tags: {}", tags);
    }
}

fn handle_status(tmp_path: &str) -> Result<(), Box<dyn Error>> {
    let path = Path::new(tmp_path);

    if !path.is_file() {
        println!("No active deep work session");
        return Ok(());
    }

    let now = Local::now();
    let record = datetime_from_last_entry(tmp_path);
    let start = DateTime::parse_from_rfc3339(&record[0])?;
    let desc = &record[1];
    let tags = &record[2];

    print_start_time(start);
    print_elapsed_time(start, now);
    print_description(desc);
    print_tags(tags);

    Ok(())
}
