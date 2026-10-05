//! Try the keyboard core from the command line.

use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use khmer_core::{Data, Engine, Style, UserDictionary, data};

const USAGE: &str = "\
usage: khmer-kbd [--data PATH] [--user FILE] COMMAND

commands:
  convert [TEXT...]                  romanized Khmer to Khmer script (default: standard input)
  suggest [-n N] [--context TEXT] TEXT...
                                     ranked candidates for the word being typed
  romanize [--style chat|ungegn] [TEXT...]
                                     Khmer script to Latin letters (default: standard input)
  repl                               type romanized Khmer and see candidates as you go
  compile EXPORT_DIR OUT_FILE        compile data/export.py output into one data file

--data   a compiled data file or an export directory (default: data/build if it
         exists, else data/sample)
--user   a file to keep learned picks in (repl: pick with :N after a suggestion)";

struct Options {
    data: Option<PathBuf>,
    user: Option<PathBuf>,
    command: String,
    rest: Vec<String>,
}

fn parse(mut args: Vec<String>) -> Result<Options, String> {
    let mut options = Options {
        data: None,
        user: None,
        command: String::new(),
        rest: Vec::new(),
    };
    while !args.is_empty() {
        let arg = args.remove(0);
        match arg.as_str() {
            "--data" | "--user" if args.is_empty() => return Err(format!("{arg} needs a value")),
            "--data" => options.data = Some(args.remove(0).into()),
            "--user" => options.user = Some(args.remove(0).into()),
            "-h" | "--help" => return Err(String::new()),
            _ => {
                options.command = arg;
                options.rest = args;
                return Ok(options);
            }
        }
    }
    Err(String::new())
}

fn load(path: Option<&Path>) -> Result<Data, String> {
    let path = path.map_or_else(
        || {
            let build = PathBuf::from("data/build");
            if build.exists() {
                build
            } else {
                PathBuf::from("data/sample")
            }
        },
        Path::to_path_buf,
    );
    let result = if path.is_dir() {
        data::compile(&path).and_then(Data::from_bytes)
    } else {
        Data::open(&path)
    };
    result.map_err(|error| format!("{}: {error}", path.display()))
}

/// The value after `flag` in `args`, removed with it.
fn take_flag(args: &mut Vec<String>, flag: &str) -> Result<Option<String>, String> {
    let Some(i) = args.iter().position(|a| a == flag) else {
        return Ok(None);
    };
    if i + 1 >= args.len() {
        return Err(format!("{flag} needs a value"));
    }
    let value = args.remove(i + 1);
    args.remove(i);
    Ok(Some(value))
}

/// The text from the arguments, or each line of standard input.
fn texts(args: &[String]) -> Vec<String> {
    if args.is_empty() {
        io::stdin().lock().lines().map_while(Result::ok).collect()
    } else {
        vec![args.join(" ")]
    }
}

fn style(name: Option<&str>) -> Result<Style, String> {
    match name {
        None | Some("chat") => Ok(Style::Chat),
        Some("ungegn") => Ok(Style::Ungegn),
        Some(other) => Err(format!("unknown style {other}; use chat or ungegn")),
    }
}

fn repl(engine: &mut Engine) -> Result<(), String> {
    println!("Type romanized Khmer. :N picks suggestion N, :k KHMER romanizes, :q quits.");
    let mut last: Option<(String, Vec<String>)> = None;
    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        if stdin
            .lock()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?
            == 0
        {
            println!();
            return Ok(());
        }
        let line = line.trim_end_matches(['\n', '\r']);
        if line == ":q" {
            return Ok(());
        }
        if let Some(khmer) = line.strip_prefix(":k ") {
            println!("  chat:   {}", engine.romanize(khmer, Style::Chat));
            println!("  ungegn: {}", engine.romanize(khmer, Style::Ungegn));
            continue;
        }
        if let Some(number) = line.strip_prefix(':').and_then(|n| n.parse::<usize>().ok()) {
            match &last {
                Some((typed, words)) if (1..=words.len()).contains(&number) => {
                    engine
                        .learn(typed, &words[number - 1])
                        .map_err(|e| e.to_string())?;
                    println!("  learned {} for {typed}", words[number - 1]);
                }
                _ => println!("  no suggestion {number}"),
            }
            continue;
        }
        if line.is_empty() {
            continue;
        }
        let result = engine.analyze(line, 5);
        println!("  {}", result.text);
        let suggestions = engine.suggest(line, 5);
        if let Some(first) = suggestions.first() {
            let typed: String = line
                .chars()
                .skip(first.start)
                .take(first.end - first.start)
                .collect();
            let words: Vec<String> = suggestions.iter().map(|s| s.text.clone()).collect();
            let shown: Vec<String> = words
                .iter()
                .enumerate()
                .map(|(i, w)| format!("{}.{w}", i + 1))
                .collect();
            println!("    {typed}: {}", shown.join("  "));
            last = Some((typed, words));
        }
    }
}

fn run(options: Options) -> Result<(), String> {
    let mut rest = options.rest;
    if options.command == "compile" {
        let [export, out] = rest.as_slice() else {
            return Err("usage: khmer-kbd compile EXPORT_DIR OUT_FILE".into());
        };
        let bytes = data::compile(Path::new(export)).map_err(|e| e.to_string())?;
        std::fs::write(out, &bytes).map_err(|e| format!("{out}: {e}"))?;
        println!("wrote {out} ({:.1} MB)", bytes.len() as f64 / 1e6);
        return Ok(());
    }
    let data = load(options.data.as_deref())?;
    let user = match &options.user {
        Some(path) => UserDictionary::open(path).map_err(|e| format!("{}: {e}", path.display()))?,
        None => UserDictionary::in_memory(),
    };
    let mut engine = Engine::with_user(data, user);
    match options.command.as_str() {
        "convert" => {
            for text in texts(&rest) {
                println!("{}", engine.convert(&text));
            }
        }
        "romanize" => {
            let style = style(take_flag(&mut rest, "--style")?.as_deref())?;
            for text in texts(&rest) {
                println!("{}", engine.romanize(&text, style));
            }
        }
        "suggest" => {
            let n = take_flag(&mut rest, "-n")?.map_or(Ok(5), |n| {
                n.parse().map_err(|_| format!("-n {n} is not a number"))
            })?;
            let context = take_flag(&mut rest, "--context")?.unwrap_or_default();
            if rest.is_empty() {
                return Err("suggest needs the text being typed".into());
            }
            for (i, s) in engine
                .suggest_in_context(&context, &rest.join(" "), n)
                .iter()
                .enumerate()
            {
                println!("{}. {}\t{:.2}\t{}", i + 1, s.text, s.score, s.origin.name());
            }
        }
        "repl" => repl(&mut engine)?,
        other => return Err(format!("unknown command {other}\n\n{USAGE}")),
    }
    Ok(())
}

fn main() -> ExitCode {
    let options = match parse(std::env::args().skip(1).collect()) {
        Ok(options) => options,
        Err(message) => {
            if !message.is_empty() {
                eprintln!("{message}\n");
            }
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
