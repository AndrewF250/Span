use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let mut seed = 0x5a1e_u64;
    let mut iters = 5_000u32;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" => seed = number(args.next(), "--seed")?,
            "--iters" => {
                let value = number(args.next(), "--iters")?;
                iters =
                    u32::try_from(value).map_err(|_| "--iters does not fit in u32".to_string())?;
            }
            "--help" => {
                println!("span-fuzzer [--seed N] [--iters N]");
                return Ok(());
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    span_protocol::exercise_random(seed, iters);
    println!("ok seed={seed} iters={iters}");
    Ok(())
}

fn number(value: Option<String>, flag: &str) -> Result<u64, String> {
    let text = value.ok_or_else(|| format!("{flag} needs a number"))?;
    text.parse::<u64>()
        .map_err(|_| format!("{flag} needs a non-negative integer"))
}
