use std::fs::File;
use std::io::{self, BufRead, BufReader, IsTerminal, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use clap::{ArgAction, Parser};
use crossterm::cursor::Show;
use crossterm::style::ResetColor;
use crossterm::QueueableCommand;
use rand::Rng;

use lolr::{animate_until, render_line, AnimateOpts, Gradient, RenderOpts};

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

#[derive(Parser, Debug)]
#[command(name = "lolr")]
#[command(about = "Rainbow colorizer for text")]
#[command(version, disable_version_flag = true)]
struct Args {
    /// Files to read (default: stdin; use - for stdin)
    #[arg()]
    files: Vec<String>,

    /// Rainbow spread
    #[arg(short = 'p', long, default_value = "3.0", value_parser = parse_minimum_f64)]
    spread: f64,

    /// Rainbow frequency
    #[arg(short = 'F', long, default_value = "0.1")]
    freq: f64,

    /// Rainbow seed (0 = random)
    #[arg(short = 'S', long, default_value = "0")]
    seed: u64,

    /// Enable psychedelics
    #[arg(short, long)]
    animate: bool,

    /// Animation duration in frames
    #[arg(short, long, default_value = "6", value_parser = parse_positive_u32)]
    duration: u32,

    /// Animation speed in frames per second
    #[arg(short, long, default_value = "40", value_parser = parse_minimum_f64)]
    speed: f64,

    /// Invert foreground and background
    #[arg(short, long)]
    invert: bool,

    /// Force 24-bit color
    #[arg(short, long)]
    truecolor: bool,

    /// Force color even when stdout is not a TTY
    #[arg(short, long)]
    force: bool,

    /// Gradient preset
    #[arg(short, long, default_value = "rainbow")]
    gradient: Gradient,

    /// Print version
    #[arg(short = 'v', short_alias = 'V', long, action = ArgAction::Version)]
    version: Option<bool>,
}

fn parse_minimum_f64(value: &str) -> Result<f64, String> {
    let value = value
        .parse::<f64>()
        .map_err(|error| format!("invalid number: {error}"))?;
    if value.is_finite() && value >= 0.1 {
        Ok(value)
    } else {
        Err("must be a finite number >= 0.1".to_owned())
    }
}

fn parse_positive_u32(value: &str) -> Result<u32, String> {
    let value = value
        .parse::<u32>()
        .map_err(|error| format!("invalid integer: {error}"))?;
    if value > 0 {
        Ok(value)
    } else {
        Err("must be >= 1".to_owned())
    }
}

fn detect_truecolor() -> bool {
    std::env::var("COLORTERM")
        .map(|value| value == "truecolor" || value == "24bit")
        .unwrap_or(false)
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let stdout = io::stdout();
    let is_stdout_tty = stdout.is_terminal();

    if !is_stdout_tty && !args.force {
        let mut stdout = stdout.lock();
        return with_inputs(&args.files, |input| {
            io::copy(input, &mut stdout)?;
            Ok(())
        });
    }

    let seed = if args.seed == 0 {
        rand::thread_rng().gen_range(0..256) as f64
    } else {
        args.seed as f64
    };

    let render_opts = RenderOpts {
        gradient: args.gradient,
        spread: args.spread,
        freq: args.freq,
        truecolor: args.truecolor || detect_truecolor(),
        invert: args.invert,
    };

    if args.animate && is_stdout_tty {
        let animate_opts = AnimateOpts {
            gradient: render_opts.gradient,
            spread: render_opts.spread,
            freq: render_opts.freq,
            seed,
            duration: args.duration,
            speed: args.speed,
            truecolor: render_opts.truecolor,
            invert: render_opts.invert,
        };
        let mut interrupt_handler_installed = false;
        return with_inputs(&args.files, |input| {
            if INTERRUPTED.load(Ordering::Relaxed) {
                return Ok(());
            }
            let mut text = String::new();
            input.read_to_string(&mut text)?;
            if !interrupt_handler_installed {
                ctrlc::set_handler(|| INTERRUPTED.store(true, Ordering::Relaxed))
                    .map_err(io::Error::other)?;
                interrupt_handler_installed = true;
            }
            animate_until(&text, &animate_opts, || INTERRUPTED.load(Ordering::Relaxed))
        });
    }

    let mut stdout = stdout.lock();
    let render_result = with_inputs(&args.files, |input| {
        // Ruby lolcat restarts the seeded gradient for each input source.
        let mut offset = seed;
        render_stream(input, &mut stdout, &render_opts, &mut offset)
    });
    let cleanup_result = if is_stdout_tty {
        stdout
            .queue(ResetColor)
            .and_then(|stdout| stdout.queue(Show))
            .and_then(Write::flush)
    } else {
        Ok(())
    };
    render_result.and(cleanup_result)
}

fn with_inputs<F>(files: &[String], mut process: F) -> io::Result<()>
where
    F: FnMut(&mut dyn BufRead) -> io::Result<()>,
{
    let stdin = io::stdin();
    if files.is_empty() {
        return process(&mut stdin.lock());
    }

    for path in files {
        if path == "-" {
            process(&mut stdin.lock())?;
        } else {
            let file = File::open(Path::new(path))?;
            process(&mut BufReader::new(file))?;
        }
    }
    Ok(())
}

fn render_stream(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    opts: &RenderOpts,
    offset: &mut f64,
) -> io::Result<()> {
    let mut line = String::new();
    loop {
        line.clear();
        if input.read_line(&mut line)? == 0 {
            return Ok(());
        }
        *offset += 1.0;
        output.write_all(render_line(&line, *offset, opts).as_bytes())?;
    }
}
