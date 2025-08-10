//! Parses a file containing the outputs of Criterion runs and prints it in CSV format to `stdout` with '|' as separator

use regex::Regex;
use std::{
    collections::BTreeMap,
    fmt::Debug,
    fs::File,
    io::{BufRead, BufReader, Lines},
};

fn cmd_line_args() -> Option<String> {
    std::env::args().nth(1)
}

fn main() {
    let infile = cmd_line_args().expect("input file must be specified as command line argument");
    let sections = parse_file(&infile);
    // println!("{sections:?}");
    for s in sections {
        print_section_to_csv(&s);
    }
}

#[derive(Debug)]
struct Time(
    /// value
    f64,
    /// unit
    String,
);

type FnTimes = BTreeMap<String, Vec<(Time, Time, Time)>>;

#[derive(Debug)]
struct Section {
    started: String,
    args: String,
    base_latency: String,
    base_effort: String,
    fn_times: FnTimes,
    finished: String,
}

type LineSrc = Lines<BufReader<File>>;

fn find_and_process_unless<T: Debug, U: Debug>(
    lines: &mut LineSrc,
    state: &mut T,
    find_pred: impl Fn(&str) -> bool,
    proc: impl Fn(&str, &mut T),
    unless_pred: impl Fn(&str) -> Option<U>,
) -> Option<U> {
    while let Some(Ok(line)) = lines.next() {
        let up = unless_pred(&line);
        if up.is_some() {
            return up;
        }

        if !find_pred(&line) {
            continue;
        }

        proc(&line, state);
    }

    None
}

fn find_and_extract<T>(
    lines: &mut LineSrc,
    find_pred: impl Fn(&str) -> bool,
    extract: impl Fn(&str) -> T,
) -> Option<T> {
    while let Some(Ok(line)) = lines.next() {
        if !find_pred(&line) {
            continue;
        }
        return Some(extract(&line));
    }
    None
}

fn parse_section(lines: &mut Lines<BufReader<File>>) -> Option<Section> {
    let started_re = Regex::new(r"^Started [^ ]+ at: \d\d:\d\d:\d\d").unwrap();
    let finished_re = Regex::new(r"^Finished [^ ]+ at: \d\d:\d\d:\d\d").unwrap();
    let args_re = Regex::new(
        r"^args=Args \{ target_ratio: (\d+(\.\d+)?), latency_unit: (\w+), base_median: (\d+(\.\d+)?), nrepeats: (\d+) \}",
    ).unwrap();
    let base_latency_re = Regex::new(r"^base_latency=(\w+)").unwrap();
    let base_effort_re = Regex::new(r"^base_effort=(\w+)").unwrap();
    let time_re = Regex::new(
        r"^([^ \[]+)[^ ]+[ ]+time:[ ]+\[(\d+(\.\d+)?)[ ]+(\w+)[ ]+(\d+(\.\d+)?)[ ]+(\w+)[ ]+(\d+(\.\d+)?)[ ]+(\w+)\]",
    )
    .unwrap();

    let started_is_match = |line: &str| started_re.is_match(line);
    let started_find = |line: &str| started_re.find(line).unwrap().as_str().to_string();

    let finished_find = |line: &str| finished_re.find(line).map(|x| x.as_str().to_string());

    let args_is_match = |line: &str| args_re.is_match(line);
    let args_find = |line: &str| args_re.find(line).unwrap().as_str().to_string();

    let base_latency_is_match = |line: &str| base_latency_re.is_match(line);
    let base_latency_find = |line: &str| base_latency_re.find(line).unwrap().as_str().to_string();

    let base_effort_is_match = |line: &str| base_effort_re.is_match(line);
    let base_effort_find = |line: &str| base_effort_re.find(line).unwrap().as_str().to_string();

    let time_is_match = |line: &str| time_re.is_match(line);
    let time_process = |line: &str, fn_times: &mut FnTimes| {
        let time_caps = time_re.captures(line).unwrap();
        let fn_name = time_caps.get(1).unwrap().as_str().to_string();
        let time0 = Time(
            time_caps.get(2).unwrap().as_str().parse().unwrap(),
            time_caps.get(4).unwrap().as_str().to_string(),
        );
        let time1 = Time(
            time_caps.get(5).unwrap().as_str().parse().unwrap(),
            time_caps.get(7).unwrap().as_str().to_string(),
        );
        let time2 = Time(
            time_caps.get(8).unwrap().as_str().parse().unwrap(),
            time_caps.get(10).unwrap().as_str().to_string(),
        );
        fn_times
            .entry(fn_name)
            .or_insert_with(|| Vec::default())
            .push((time0, time1, time2));
    };

    // Short-circuit if can't find "Started".
    let started = find_and_extract(lines, started_is_match, started_find)?;
    let args = find_and_extract(lines, args_is_match, args_find).unwrap();
    let base_latency = find_and_extract(lines, base_latency_is_match, base_latency_find).unwrap();
    let base_effort = find_and_extract(lines, base_effort_is_match, base_effort_find).unwrap();

    let mut fn_times = FnTimes::default();
    let finished = find_and_process_unless(
        lines,
        &mut fn_times,
        time_is_match,
        time_process,
        finished_find,
    )
    .unwrap();

    Some(Section {
        started,
        args,
        base_latency,
        base_effort,
        fn_times,
        finished,
    })
}

fn parse_file(infile: &str) -> Vec<Section> {
    // Open the file
    let file = File::open(infile).expect("Failed to open file");
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let mut sections = Vec::<Section>::new();

    loop {
        if let Some(section) = parse_section(&mut lines) {
            sections.push(section);
        } else {
            break;
        }
    }

    sections
}

/// Prints a [`Section`] in CSV format to stdout with '|' as separator.
fn print_section_to_csv(s: &Section) {
    println!("\n>>> Section - {}", s.started);
    println!();
    println!("{}", s.args);
    println!("{}, {}", s.base_latency, s.base_effort);
    println!();

    let nkeys = s.fn_times.keys().len();
    let nrows = s.fn_times.iter().next().unwrap().1.len();

    for j in 0..nkeys {
        print!("fn_name|lo_time|unit|mid_time|unit|hi_time|unit");
        if j < nkeys - 1 {
            print!("| |")
        } else {
            println!();
        }
    }

    let mut time_table = Vec::<&Vec<(Time, Time, Time)>>::new();
    let mut name_vec = Vec::<&String>::new();
    {
        for k in s.fn_times.keys() {
            time_table.push(s.fn_times.get(k).unwrap());
            name_vec.push(k);
        }
    }

    for i in 0..nrows {
        for j in 0..nkeys {
            print!(
                "{}|{}|{}|{}|{}|{}|{}",
                name_vec[j],
                time_table[j][i].0.0,
                time_table[j][i].0.1,
                time_table[j][i].1.0,
                time_table[j][i].1.1,
                time_table[j][i].2.0,
                time_table[j][i].2.1,
            );
            if j < nkeys - 1 {
                print!("| |")
            } else {
                println!();
            }
        }
    }

    println!("\n<<< {}", s.finished);
}
