extern crate chrono;
extern crate docopt;
extern crate hyper;
extern crate regex;
extern crate rustc_serialize;

use docopt::Docopt;
use regex::Regex;
use std::collections::HashMap;

mod api2;
mod bzapi;

#[cfg_attr(rustfmt, rustfmt_skip)]
const USAGE: &'static str = "
Standups Weekly Report.

Usage:
  standups_weekly [-w] [-d <date>]
  standups_weekly [-w] -s <start_date> -e <end_date>
  standups_weekly -h

Options:
  -h --help                  Show this screen.
  -d <date>, --date <date>   The date in yyyy-mm-dd format.
  -s <date>, --start <date>  The date in yyyy-mm-dd format.
  -e <date>, --end <date>    The date in yyyy-mm-dd format.
  -w --wiki                  Output report in mediawiki format.
";

fn titlecase(input: &str) -> String {
    input.chars()
         .enumerate()
         .map(|(i, c)| {
             if i == 0 {
                 c.to_uppercase().next().unwrap()
             } else {
                 c
             }
         })
         .collect()
}

fn textify(maybe_html: &str) -> String {
    let bug_re = Regex::new("<a href=\"http://bugzilla[^\"]+\">[Bb]ug\\s+(?P<number>\\d+)</a>")
                     .unwrap();
    let text = bug_re.replace_all(maybe_html, "$number");

    let bug_re = Regex::new("(?P<number>\\d{5,})").unwrap();
    bug_re.replace_all(&text, "bug $number")
}

fn extract_bug_numbers(input: &str) -> Vec<String> {
    let bug_re = Regex::new("[Bb]ug\\s+(?P<number>\\d+)").unwrap();
    bug_re.captures_iter(input)
          .map(|caps| caps.name("number").unwrap().to_string())
          .collect()
}

fn extract_bug_details(bugs: &Vec<String>) -> Vec<String> {
    let mut result = Vec::new();
    for bug_number in bugs {
        let data = bzapi::get_bug_data(&bug_number);
        result.push(format!("https://bugzil.la/{} {}", bug_number, data));
    }
    result
}

fn print_section(section: &str, wiki: bool) {
    if wiki {
        println!("\n== {} ==", section);
    } else {
        println!("\n## {} ##", section);
    }
}

fn main() {
    let args = Docopt::new(USAGE)
                   .and_then(|dopt| dopt.parse())
                   .unwrap_or_else(|e| e.exit());

    let date = args.get_str("--date");
    let week_start = args.get_str("--start");
    let week_end = args.get_str("--end");
    let wiki = args.get_bool("--wiki");
    let decoded;
    if !date.is_empty() {
        decoded = api2::get_project_timeline("perf-tw", &date);
    } else {
        decoded = api2::get_project_timeline_range("perf-tw", &week_start, &week_end);
    }

    let mut reports = HashMap::new();

    for status in &decoded {
        let vec = reports.entry(&status.user.name).or_insert(Vec::new());
        vec.push(titlecase(&textify(&status.content)));
    }

    for (username, status) in reports.iter_mut() {
        status.sort();
        status.dedup();

        print_section(username, wiki);
        if wiki {
            let mut bugs_map = HashMap::new();
            for content in status {
                let bugs = extract_bug_numbers(&content);
                for bug in bugs {
                    let vec = bugs_map.entry(bug).or_insert(Vec::new());
                    vec.push(content.clone());
                }
            }
            for (bug, vec) in bugs_map.iter() {
                let bug_detail = bzapi::get_bug_data(&bug);
                println!("* {{{{{}}}}} {}", bug, bug_detail);
                for content in vec {
                    println!("** {}", content);
                }
            }
        } else {
            let mut bugs = Vec::new();
            for content in status {
                println!("  * {}", content);
                bugs.extend(extract_bug_numbers(&content));
            }
            if !bugs.is_empty() {
                println!("");
                bugs.sort();
                bugs.dedup();
                let bugs_detail = extract_bug_details(&bugs);
                for bug in bugs_detail {
                    println!("  * {}", bug);
                }
            }
        }
    }

    if wiki {
        println!("\n\n<small>");
    } else {
        println!("\n\n");
    }
    println!("This report is automatically generated by \
              https://github.com/kanru/standups_weekly");
    if wiki {
        println!("</small>");
    }
}
