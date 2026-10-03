use std::fs::File;
use std::io::{self, BufRead, BufReader, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use clap::parser::ValueSource;
use clap::{ArgAction, CommandFactory, FromArgMatches, Parser};
use crossterm::cursor::Show;
use crossterm::style::ResetColor;
use crossterm::QueueableCommand;

use lolr::{
    animate_reader_styled_until, render_line_styled, AnimateOpts, AnimationDirection, Background,
    Gradient, RenderOpts, RenderStyle, Renderer,
};

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

    /// Use a named palette from the config file
    #[arg(long, value_name = "NAME", conflicts_with = "gradient")]
    palette: Option<String>,

    /// Adjust colors for a dark or light terminal background
    #[arg(long, value_enum, conflicts_with_all = ["no_background", "invert"])]
    background: Option<Background>,

    /// Disable background contrast configured in the config file
    #[arg(long)]
    no_background: bool,

    /// Reduce repeated ANSI color codes (output bytes differ from lolcat)
    #[arg(long, conflicts_with = "no_compact")]
    compact: bool,

    /// Disable compact output configured in the config file
    #[arg(long)]
    no_compact: bool,

    /// Animation direction
    #[arg(long, value_enum, default_value = "forward")]
    direction: AnimationDirection,

    /// List available gradient presets and exit
    #[arg(long)]
    list_gradients: bool,

    /// Preview built-in gradients or one named palette and exit
    #[arg(long, num_args = 0..=1, default_missing_value = "all", value_name = "NAME", conflicts_with = "files")]
    preview: Option<String>,

    /// Continue the gradient across input files
    #[arg(long, conflicts_with = "no_continuous")]
    continuous: bool,

    /// Restart the gradient for each file
    #[arg(long)]
    no_continuous: bool,

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

fn no_color_requested() -> bool {
    std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty())
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

    if let Some(choice) = args.preview.as_deref() {
        let stdout = io::stdout();
        let is_tty = stdout.is_terminal();
        let color = (is_tty || args.force)
            && (!no_color_requested()
                || matches.value_source("force") == Some(ValueSource::CommandLine));
        let opts = RenderOpts {
            gradient: Gradient::Rainbow,
            spread: args.spread,
            freq: args.freq,
            truecolor: args.truecolor || (!args.no_truecolor && detect_truecolor()),
            invert: args.invert,
        };
        let style = RenderStyle {
            palette: None,
            background: args.background,
            compact: args.compact,
        };
        let mut stdout = stdout.lock();
        let gradients: Vec<Gradient> = if choice == "all" {
            Gradient::ALL.to_vec()
        } else {
            Gradient::from_name(choice).into_iter().collect()
        };
        for gradient in gradients {
            let sample = format!(
                "{:<10} The quick brown fox jumps over the lazy dog",
                gradient.as_str()
            );
            if color {
                writeln!(
                    stdout,
                    "{}",
                    render_line_styled(
                        &sample,
                        1.0,
                        &RenderOpts {
                            gradient,
                            ..opts.clone()
                        },
                        &style
                    )
                )?;
            } else {
                writeln!(stdout, "{sample}")?;
            }
        }
        if choice != "all" && Gradient::from_name(choice).is_none() {
            let config = if args.no_config {
                Config::default()
            } else {
                let path = Config::resolve_path(args.config.as_deref())?;
                Config::load_if_exists(&path)?
            };
            let palette = config.palette_named(choice)?;
            let sample = format!("{choice:<10} The quick brown fox jumps over the lazy dog");
            if color {
                let style = RenderStyle {
                    palette: Some(palette),
                    ..style
                };
                writeln!(
                    stdout,
                    "{}",
                    render_line_styled(&sample, 1.0, &opts, &style)
                )?;
            } else {
                writeln!(stdout, "{sample}")?;
            }
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

    let config = if let Some(path) = config_path {
        Config::load_or_create(&path)?
    } else {
        Config::default()
    };
    apply_config(&mut args, &matches, &config);

    if args.invert && args.background.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--background cannot be combined with --invert: contrast adjustment applies to foreground colors",
        ));
    }

    let style = RenderStyle {
        palette: args
            .palette
            .as_deref()
            .map(|name| config.palette_named(name))
            .transpose()?,
        background: args.background,
        compact: args.compact,
    };

    let stdout = io::stdout();
    let is_stdout_tty = stdout.is_terminal();

    if !args.force && (!is_stdout_tty || no_color_requested()) {
        let mut stdout = stdout.lock();
        return with_inputs(&args.files, |input| {
            io::copy(input, &mut stdout)?;
            Ok(())
        });
    }

    let seed = if args.seed == 0 {
        rand::random_range(0..256) as f64
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
        let mut line_offset = seed;
        return with_inputs(&args.files, |input| {
            if INTERRUPTED.load(Ordering::Relaxed) {
                return Ok(());
            }
            if !interrupt_handler_installed {
                ctrlc::set_handler(|| INTERRUPTED.store(true, Ordering::Relaxed))
                    .map_err(io::Error::other)?;
                interrupt_handler_installed = true;
            }
            if !args.continuous {
                line_offset = seed;
            }
            animate_reader_styled_until(
                input,
                &animate_opts,
                &style,
                args.direction,
                &mut line_offset,
                || INTERRUPTED.load(Ordering::Relaxed),
            )
        });
    }

    let mut stdout = stdout.lock();
    let mut offset = seed;
    let render_result = with_inputs(&args.files, |input| {
        // Ruby lolcat restarts the seeded gradient for each input source.
        if !args.continuous {
            offset = seed;
        }
        render_stream(input, &mut stdout, &render_opts, &style, &mut offset)
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

fn apply_config(args: &mut Args, matches: &clap::ArgMatches, config: &Config) {
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
    if !from_cli("palette") && !from_cli("gradient") {
        args.palette = config.palette.clone();
    }
    if from_cli("no_background") {
        args.background = None;
    } else if !from_cli("background") {
        args.background = config.background;
    }
    if !from_cli("direction") {
        args.direction = config.direction;
    }
    if from_cli("no_continuous") {
        args.continuous = false;
    } else if !from_cli("continuous") {
        args.continuous = config.continuous;
    }
    if from_cli("no_compact") {
        args.compact = false;
    } else if !from_cli("compact") {
        args.compact = config.compact;
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
    style: &RenderStyle,
    offset: &mut f64,
) -> io::Result<()> {
    let mut line = Vec::new();
    let mut rendered = Vec::new();
    let mut renderer = Renderer::default();
    loop {
        line.clear();
        if input.read_until(b'\n', &mut line)? == 0 {
            return Ok(());
        }
        *offset += 1.0;
        match std::str::from_utf8(&line) {
            Ok(text) => {
                renderer.render_into(text, *offset, opts, style, &mut rendered);
                output.write_all(&rendered)?
            }
            Err(_) => output.write_all(&line)?,
        }
    }
}
