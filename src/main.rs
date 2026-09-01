use std::fs;
use std::io::{self, BufRead, IsTerminal, Read};

use clap::Parser;
use rand::Rng;

use lolr::{animate, render_line, AnimateOpts, Gradient, RenderOpts};

#[derive(Parser, Debug)]
#[command(name = "lolr")]
#[command(about = "Rainbow colorizer for text")]
#[command(version)]
struct Args {
    /// Files to read (default: stdin)
    #[arg()]
    files: Vec<String>,

    /// Rainbow spread
    #[arg(short = 'p', long, default_value = "3.0")]
    spread: f64,

    /// Rainbow frequency
    #[arg(short = 'F', long, default_value = "0.1")]
    freq: f64,

    /// Color seed (0 = random)
    #[arg(short = 'S', long, default_value = "0")]
    seed: u64,

    /// Enable animation
    #[arg(short, long)]
    animate: bool,

    /// Animation frames
    #[arg(short, long, default_value = "12")]
    duration: u32,

    /// Animation FPS
    #[arg(short, long, default_value = "20")]
    speed: f64,

    /// Swap foreground/background
    #[arg(short, long)]
    invert: bool,

    /// Force 24-bit color
    #[arg(short, long)]
    truecolor: bool,

    /// Force color on non-TTY
    #[arg(short, long)]
    force: bool,

    /// Gradient preset
    #[arg(short, long, default_value = "rainbow")]
    gradient: String,
}

fn detect_truecolor() -> bool {
    std::env::var("COLORTERM")
        .map(|v| v == "truecolor" || v == "24bit")
        .unwrap_or(false)
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    let gradient = Gradient::from_name(&args.gradient).unwrap_or(Gradient::Rainbow);

    let truecolor = args.truecolor || detect_truecolor();

    let seed = if args.seed == 0 {
        rand::thread_rng().gen_range(0.0..256.0)
    } else {
        args.seed as f64
    };

    let stdout = io::stdout();
    let stdin = io::stdin();
    let is_stdout_tty = stdout.is_terminal();

    if !is_stdout_tty && !args.force {
        // No color: pass through
        if args.files.is_empty() {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            print!("{}", buffer);
        } else {
            print!("{}", read_files(&args.files)?);
        }
        return Ok(());
    }

    let opts = RenderOpts {
        gradient,
        spread: args.spread,
        freq: args.freq,
        truecolor,
        invert: args.invert,
    };

    // Stdin: process line by line (matches Ruby lolcat behavior)
    if args.files.is_empty() {
        let mut offset = seed;
        for line in stdin.lock().lines() {
            let line = line?;
            if args.animate && is_stdout_tty {
                let anim_opts = AnimateOpts {
                    gradient,
                    spread: args.spread,
                    freq: args.freq,
                    seed: offset,
                    duration: args.duration,
                    speed: args.speed,
                    truecolor,
                    invert: args.invert,
                };
                animate(&line, &anim_opts)?;
            } else {
                let colored = render_line(&line, offset, &opts);
                println!("{}", colored);
            }
            offset += 1.0;
        }
        return Ok(());
    }

    // Files: read all then process
    let text = read_files(&args.files)?;

    if args.animate && is_stdout_tty {
        // Animate file content line by line too
        let mut offset = seed;
        for line in text.lines() {
            let anim_opts = AnimateOpts {
                gradient,
                spread: args.spread,
                freq: args.freq,
                seed: offset,
                duration: args.duration,
                speed: args.speed,
                truecolor,
                invert: args.invert,
            };
            animate(line, &anim_opts)?;
            offset += 1.0;
        }
    } else {
        let mut offset = seed;
        for line in text.lines() {
            let colored = render_line(line, offset, &opts);
            println!("{}", colored);
            offset += 1.0;
        }
    }

    Ok(())
}

fn read_files(files: &[String]) -> io::Result<String> {
    let mut combined = String::new();
    for path in files {
        let content = fs::read_to_string(path)?;
        combined.push_str(&content);
    }
    Ok(combined)
}
