use std::env;
use std::process::Command;

use mayberun::*;

fn main() {
    let input_glob = env::var("IN").ok();
    let output_glob = env::var("OUT").ok();

    let args: Vec<String> = env::args().skip(1).collect();
    let head: Option<&String> = args.first();

    match head {
        None => {}
        Some(cmd) => {
            let before = if let Some(input) = input_glob {
                check_glob(&input)
            } else {
                CheckResult::Changed
            };

            let (after, out_glob) = if let Some(output) = output_glob {
                (check_glob(&output), Some(output))
            } else {
                (CheckResult::Changed, None)
            };

            if before == CheckResult::Changed || after == CheckResult::Changed {
                let tail: &[String] = &args[1..];
                let mut child = Command::new(cmd).args(tail).spawn().expect("Failed");
                let result = child.wait();
                if let Some(out) = out_glob {
                    match result {
                        Ok(_) => write_glob(&out),
                        Err(_) => {}
                    }
                }
            }
        }
    }
}
