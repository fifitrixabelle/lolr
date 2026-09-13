use std::fs::File;
use std::io::{self, BufRead, BufReader, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use clap::parser::ValueSource;
use clap::{ArgAction, CommandFactory, FromArgMatches, Parser};
use crossterm::cursor::Show;
use crossterm::style::ResetColor;
use crossterm::QueueableCommand;
use rand::Rng;

use lolr::{animate_until, render_line, AnimateOpts, Gradient, RenderOpts};

mod config;

use config::{Config, DEFAULT_DURATION, DEFAULT_FREQ, DEFAULT_SEED, DEFAULT_SPEED, DEFAULT_SPREAD};

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

#[derive(Parser, Debug)]
#[command(name = "lolr")]
#[command(about = "Rainbow colorizer for text")]
#[command(version, disable_version_flag = true)]
struct Args {
    /// Files to read (default: stdin; use - for stdin)
    #[arg()]
    files: Vec<String>,

    /// Use a specific config file
    #[arg(long, value_name = "PATH", conflicts_with = "no_config")]
    config: Option<PathBuf>,

    /// Ignore the config file and do not create one
    #[arg(long, conflicts_with_all = ["config", "print_config_path"])]
    no_config: bool,

    /// Print the resolved config path and exit
    #[arg(long, conflicts_with = "no_config")]
    print_config_path: bool,

    /// Rainbow spread
    #[arg(short = 'p', long, default_value_t = DEFAULT_SPREAD, value_parser = parse_minimum_f64)]
    spread: f64,

    /// Rainbow frequency
    #[arg(short = 'F', long, default_value_t = DEFAULT_FREQ, value_parser = parse_finite_f64)]
    freq: f64,

    /// Rainbow seed (0 = random)
    #[arg(short = 'S', long, default_value_t = DEFAULT_SEED)]
    seed: u64,

    /// Enable psychedelics
    #[arg(short, long, conflicts_with = "no_animate")]
    animate: bool,

    /// Disable animation configured in the config file
    #[arg(long)]
    no_animate: bool,

    /// Animation duration in frames
    #[arg(short, long, default_value_t = DEFAULT_DURATION, value_parser = parse_positive_u32)]
    duration: u32,

    /// Animation speed in frames per second
    #[arg(short, long, default_value_t = DEFAULT_SPEED, value_parser = parse_minimum_f64)]
    speed: f64,

    /// Invert foreground and background
    #[arg(short, long, conflicts_with = "no_invert")]
    invert: bool,

    /// Disable inversion configured in the config file
    #[arg(long)]
    no_invert: bool,

    /// Force 24-bit color
    #[arg(short, long, conflicts_with = "no_truecolor")]
    truecolor: bool,

    /// Disable 24-bit color, including automatic detection
    #[arg(long)]
    no_truecolor: bool,

    /// Force color even when stdout is not a TTY
    #[arg(short, long, conflicts_with = "no_force")]
    force: bool,

    /// Disable forced color configured in the config file
    #[arg(long)]
    no_force: bool,

    /// Gradient preset (available: rainbow, fire, ocean, pastel, neon, sunset, forest, synthwave, viridis, aura)
    #[arg(short, long, default_value_t = Gradient::default())]
    gradient: Gradient,

    /// List available gradient presets and exit
    #[arg(long)]
    list_gradients: bool,

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

fn parse_finite_f64(value: &str) -> Result<f64, String> {
    let value = value
        .parse::<f64>()
        .map_err(|error| format!("invalid number: {error}"))?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err("must be a finite number".to_owned())
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
    let matches = Args::command().get_matches();
    let mut args = Args::from_arg_matches(&matches).expect("arguments should be valid");

    if args.list_gradients {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        for gradient in Gradient::ALL {
            writeln!(stdout, "{}", gradient.as_str())?;
        }
        return Ok(());
    }

    let config_path = if args.no_config {
        None
    } else {
        Some(Config::resolve_path(args.config.as_deref())?)
    };

    if args.print_config_path {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        writeln!(
            stdout,
            "{}",
            config_path
                .expect("config path should be resolved")
                .display()
        )?;
        return Ok(());
    }

    if let Some(path) = config_path {
        let config = Config::load_or_create(&path)?;
        apply_config(&mut args, &matches, config);
    }

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
        truecolor: args.truecolor || (!args.no_truecolor && detect_truecolor()),
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

fn apply_config(args: &mut Args, matches: &clap::ArgMatches, config: Config) {
    let from_cli = |name| matches.value_source(name) == Some(ValueSource::CommandLine);

    if !from_cli("spread") {
        args.spread = config.spread;
    }
    if !from_cli("freq") {
        args.freq = config.freq;
    }
    if !from_cli("seed") {
        args.seed = config.seed;
    }
    if from_cli("no_animate") {
        args.animate = false;
    } else if !from_cli("animate") {
        args.animate = config.animate;
    }
    if !from_cli("duration") {
        args.duration = config.duration;
    }
    if !from_cli("speed") {
        args.speed = config.speed;
    }
    if from_cli("no_invert") {
        args.invert = false;
    } else if !from_cli("invert") {
        args.invert = config.invert;
    }
    if from_cli("no_truecolor") {
        args.truecolor = false;
    } else if !from_cli("truecolor") {
        args.truecolor = config.truecolor;
    }
    if from_cli("no_force") {
        args.force = false;
    } else if !from_cli("force") {
        args.force = config.force;
    }
    if !from_cli("gradient") {
        args.gradient = config.gradient;
    }
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
