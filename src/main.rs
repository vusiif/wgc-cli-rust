mod capture;
mod cli;
mod output;

use std::fs;
use std::process::ExitCode;
use std::thread::sleep;
use std::time::Duration;
use clap::Parser;
use cli::Cli;
use log::debug;
use output::{ApiError, ApiResponse, CaptureResultJson, ErrorCode, OutputDestination};

fn main() -> ExitCode {
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

    let is_json = args.json;
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            let exit_code = err.exit_code;
            if is_json {
                let response = ApiResponse::<()>::failure(err);
                if let Ok(json_str) = serde_json::to_string_pretty(&response) {
                    println!("{}", json_str);
                } else {
                    println!(r#"{{"ok":false,"error":{{"errorCode":"UNKNOWN","message":"Failed to serialize error payload","exitCode":127}}}}"#);
                }
            } else {
                eprintln!("Error: {}", err.message);
                if let Some(sug) = err.suggestion {
                    eprintln!("Suggestion: {}", sug);
                }
            }
            ExitCode::from(exit_code as u8)
        }
    }
}

fn run(args: Cli) -> Result<(), ApiError> {
    // 1. Determine output destination and validate arguments early
    let destination = output::determine_output_destination(
        args.output.as_deref(),
        args.output_file.as_deref(),
        args.output_dir.as_deref(),
    )?;

    let is_stdout = destination == OutputDestination::Stdout;

    // R-01: Prohibit combining --json with -o - (stdout) to prevent stream corruption
    if is_stdout && args.json {
        return Err(ApiError::new(
            ErrorCode::BadArgs,
            "Cannot combine '--json' with standard output streaming ('-o -'). \
             Binary image stream and structured JSON text cannot both occupy stdout.",
        )
        .with_stage("argument_validation")
        .with_suggestion("To retrieve structured JSON metadata, save to a file or directory instead of stdout."));
    }

    // Infer image format and validate against file extension if explicit format provided
    let format = output::validate_and_infer_format(&destination, args.format)?;

    // 2. Handle monitor listing if requested
    if args.list_monitors {
        let monitors = capture::get_monitor_infos()?;

        if args.json {
            let response = ApiResponse::success(monitors);
            let json_str = serde_json::to_string_pretty(&response).map_err(|e| {
                ApiError::new(ErrorCode::EncodeFailed, format!("JSON serialization failed: {e}"))
            })?;
            println!("{}", json_str);
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

    // 3. Handle delayed capture if requested
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

    // 4. Perform capture with monitor selection & optional region cropping
    debug!(
        "Initiating capture (monitor={:?}, region={:?})",
        args.monitor, args.region
    );
    let dynamic_image = capture::capture_screen(args.monitor, args.region.as_ref())?;
    let img_width = dynamic_image.width();
    let img_height = dynamic_image.height();

    // 5. Handle clipboard export if requested
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

    // 6. Output handling:
    let mut saved_path_buf = None;
    let mut file_size_bytes = None;

    if is_stdout {
        output::write_to_stdout(&dynamic_image, format)?;
    } else if destination != OutputDestination::Auto || !args.clipboard {
        let saved_path = output::save_image(&dynamic_image, &destination, format)?;

        if let Ok(meta) = fs::metadata(&saved_path) {
            file_size_bytes = Some(meta.len());
        }

        if !args.quiet && !args.json {
            println!("Screenshot successfully saved to: {}", saved_path.display());
        }

        saved_path_buf = Some(saved_path);
    }

    // 7. Handle JSON output mode
    if args.json {
        let result_payload = CaptureResultJson {
            status: "success".to_string(),
            path: saved_path_buf.map(|p| p.display().to_string()),
            width: img_width,
            height: img_height,
            format,
            clipboard: args.clipboard,
            size_bytes: file_size_bytes,
        };
        let response = ApiResponse::success(result_payload);
        let json_str = serde_json::to_string_pretty(&response).map_err(|e| {
            ApiError::new(ErrorCode::EncodeFailed, format!("JSON serialization failed: {e}"))
        })?;
        println!("{}", json_str);
    }

    Ok(())
}
