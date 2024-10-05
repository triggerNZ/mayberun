use std::process::Command;
use std::{env, io};

use mayberun::*;

fn main() -> io::Result<()> {
    let input_glob = env::var("IN").ok();
    let output_glob = env::var("OUT").ok();

    let args: Vec<String> = env::args().skip(1).collect();
    let head: Option<&String> = args.first();

    let cwd = env::current_dir()?;

    match head {
        None => (),
        Some(cmd) => {
            let (before, in_glob) = if let Some(input) = input_glob {
                (
                    check_glob(&env::current_dir().unwrap(), &input)?,
                    Some(input),
                )
            } else {
                (CheckResult::Changed, None)
            };

            let (after, out_glob) = if let Some(output) = output_glob {
                (Some(check_glob(&cwd, &output)?), Some(output))
            } else {
                (None, None)
            };

            if before == CheckResult::Changed || after == Some(CheckResult::Changed) {
                println!("before: {:?}, after: {:?}", before, after);
                let tail: &[String] = &args[1..];
                let mut child = Command::new(cmd).args(tail).spawn().expect("Failed");
                let result = child.wait();
                if let Some(inp) = in_glob {
                    match result {
                        Ok(_) => write_glob(&cwd, &inp)?,
                        Err(_) => (),
                    }
                }

                if let Some(out) = out_glob {
                    match result {
                        Ok(_) => write_glob(&cwd, &out)?,
                        Err(_) => (),
                    }
                }
            }
        }
    }
    Ok(())
}
