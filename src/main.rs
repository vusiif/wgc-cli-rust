mod capture;
mod cli;
mod output;

use std::fs;
use std::thread::sleep;
use std::time::Duration;
use clap::Parser;
use cli::Cli;
use log::debug;
use output::CaptureResultJson;

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let log_level = if args.verbose {
        "debug"
    } else if args.quiet {
        "error"
    } else {
        "warn"
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level))
        .format_timestamp(None)
        .format_target(false)
        .init();

    debug!("Parsed CLI arguments: {:?}", args);
    run(args)
}

fn run(args: Cli) -> anyhow::Result<()> {
    // 1. Handle monitor listing if requested
    if args.list_monitors {
        let monitors = capture::get_monitor_infos()?;

        if args.json {
            println!("{}", serde_json::to_string_pretty(&monitors)?);
            return Ok(());
        }

        if monitors.is_empty() {
            println!("No monitors detected.");
            return Ok(());
        }

        println!("Available Monitors (total: {}):", monitors.len());
        println!("{:-<60}", "");
        for m in monitors {
            let primary_tag = if m.is_primary { " [Primary]" } else { "" };
            println!("[{}] {}{}", m.index, m.name, primary_tag);
            println!("    Resolution:   {} x {}", m.width, m.height);
            println!("    Scale Factor: {:.2}x", m.scale_factor);
            println!("{:-<60}", "");
        }
        return Ok(());
    }

    // Determine if output is destined for stdout piping (`-o -`)
    let is_stdout = args.output.as_deref().is_some_and(|p| p.to_str() == Some("-"));

    // Infer image format from explicit flag or destination file extension
    let format = output::infer_format(args.output.as_deref(), args.format);

    // 2. Handle delayed capture if requested
    if let Some(delay_ms) = args.delay {
        if delay_ms > 0 {
            if !args.quiet && !args.json {
                if is_stdout {
                    eprintln!("Waiting {} ms before capturing...", delay_ms);
                } else {
                    println!("Waiting {} ms before capturing...", delay_ms);
                }
            }
            sleep(Duration::from_millis(delay_ms));
        }
    }

    // 3. Perform capture with monitor selection & optional region cropping
    debug!(
        "Initiating capture (monitor={:?}, region={:?})",
        args.monitor, args.region
    );
    let dynamic_image = capture::capture_screen(args.monitor, args.region.as_ref())?;
    let img_width = dynamic_image.width();
    let img_height = dynamic_image.height();

    // 4. Handle clipboard export if requested
    if args.clipboard {
        output::copy_to_clipboard(&dynamic_image)?;
        if !args.quiet && !args.json {
            if is_stdout {
                eprintln!("Screenshot successfully copied to clipboard.");
            } else {
                println!("Screenshot successfully copied to clipboard.");
            }
        }
    }

    // 5. Output handling:
    let mut saved_path_buf = None;
    let mut file_size_bytes = None;

    if is_stdout {
        output::write_to_stdout(&dynamic_image, format)?;
    } else if args.output.is_some() || !args.clipboard {
        let saved_path = output::save_image(
            &dynamic_image,
            args.output.as_deref(),
            format,
        )?;

        if let Ok(meta) = fs::metadata(&saved_path) {
            file_size_bytes = Some(meta.len());
        }

        if !args.quiet && !args.json {
            println!("Screenshot successfully saved to: {}", saved_path.display());
        }

        saved_path_buf = Some(saved_path);
    }

    // 6. Handle JSON output mode
    if args.json {
        let json_output = CaptureResultJson {
            status: "success".to_string(),
            path: saved_path_buf.map(|p| p.display().to_string()),
            width: img_width,
            height: img_height,
            format,
            clipboard: args.clipboard,
            size_bytes: file_size_bytes,
        };
        println!("{}", serde_json::to_string_pretty(&json_output)?);
    }

    Ok(())
}
