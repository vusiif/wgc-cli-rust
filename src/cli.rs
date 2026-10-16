use std::path::PathBuf;
use std::str::FromStr;
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};

/// Representation of a rectangular screen region (logical coordinates).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl FromStr for Region {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(',').map(|p| p.trim()).collect();
        if parts.len() != 4 {
            return Err("Region must be formatted as 'x,y,w,h' (e.g. 100,200,800,600)".to_string());
        }

        let x = parts[0]
            .parse::<i32>()
            .map_err(|e| format!("Invalid x coordinate: {e}"))?;
        let y = parts[1]
            .parse::<i32>()
            .map_err(|e| format!("Invalid y coordinate: {e}"))?;
        let width = parts[2]
            .parse::<u32>()
            .map_err(|e| format!("Invalid width: {e}"))?;
        let height = parts[3]
            .parse::<u32>()
            .map_err(|e| format!("Invalid height: {e}"))?;

        if width == 0 || height == 0 {
            return Err("Width and height of region must be greater than 0".to_string());
        }

        Ok(Region {
            x,
            y,
            width,
            height,
        })
    }
}

/// Coordinate space for region cropping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CoordMode {
    /// Logical coordinates scaled by monitor DPI (default)
    #[default]
    Logical,
    /// Exact physical pixel coordinates (1:1 without DPI scaling)
    Physical,
}

/// Supported output image formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default, Serialize, Deserialize)]
pub enum ImageFormat {
    #[default]
    Png,
    Jpeg,
    Webp,
}

impl ImageFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Webp => "webp",
        }
    }
}

/// Cross-platform command-line screen capture utility.
#[derive(Parser, Debug)]
#[command(
    name = "rgc-cli",
    version,
    about = "Cross-platform screen capture CLI utility written in Rust",
    long_about = "Fast, lightweight, and cross-platform screen capture utility supporting multi-monitor selection, high-DPI scaling, clipboard export, and stdout streaming."
)]
pub struct Cli {
    /// Run diagnostic environment check and inspect capture capabilities
    #[arg(long = "doctor", conflicts_with_all = ["mcp", "list_monitors", "monitor", "region"])]
    pub doctor: bool,

    /// Start Model Context Protocol (MCP) stdio server for AI agent automation
    #[arg(long = "mcp", conflicts_with_all = ["doctor", "list_monitors", "monitor", "region", "output", "output_file", "output_dir", "clipboard"])]
    pub mcp: bool,

    /// List all available monitors and exit
    #[arg(short = 'l', long = "list-monitors")]
    pub list_monitors: bool,

    /// Monitor index to capture (0-indexed). Defaults to primary monitor
    #[arg(short = 'm', long = "monitor", value_name = "INDEX")]
    pub monitor: Option<usize>,

    /// Region to capture in coordinates: 'x,y,w,h' (e.g. 100,100,800,600)
    #[arg(short = 'r', long = "region", value_name = "x,y,w,h")]
    pub region: Option<Region>,

    /// Coordinate system for region cropping: 'logical' (scaled by monitor DPI) or 'physical' (exact pixels)
    #[arg(long = "coords", value_enum, default_value_t = CoordMode::Logical)]
    pub coords: CoordMode,

    /// Output file path or directory, or '-' for stdout stream. Defaults to timestamped file in CWD
    #[arg(
        short = 'o',
        long = "output",
        value_name = "PATH",
        conflicts_with_all = ["output_file", "output_dir"]
    )]
    pub output: Option<PathBuf>,

    /// Explicit output file destination (creates parent directories if missing)
    #[arg(
        long = "output-file",
        value_name = "FILE",
        conflicts_with_all = ["output", "output_dir"]
    )]
    pub output_file: Option<PathBuf>,

    /// Explicit output directory destination (timestamped filename generated inside)
    #[arg(
        long = "output-dir",
        value_name = "DIR",
        conflicts_with_all = ["output", "output_file"]
    )]
    pub output_dir: Option<PathBuf>,

    /// Copy screenshot directly to system clipboard
    #[arg(short = 'c', long = "clipboard")]
    pub clipboard: bool,

    /// Delay before capturing in milliseconds (e.g. 1000 for 1 second)
    #[arg(short = 'd', long = "delay", value_name = "MS")]
    pub delay: Option<u64>,

    /// Output image format. If omitted, automatically inferred from file extension or defaults to PNG
    #[arg(short = 'f', long = "format", value_enum)]
    pub format: Option<ImageFormat>,

    /// Output result as structured JSON (ideal for automation & AI agents)
    #[arg(long = "json")]
    pub json: bool,

    /// Enable verbose / debug logging
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Suppress non-error output
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_region_valid() {
        let region = "100,200,800,600".parse::<Region>().unwrap();
        assert_eq!(
            region,
            Region {
                x: 100,
                y: 200,
                width: 800,
                height: 600
            }
        );
    }

    #[test]
    fn test_parse_region_with_spaces() {
        let region = " 50 , -10 , 1920 , 1080 ".parse::<Region>().unwrap();
        assert_eq!(
            region,
            Region {
                x: 50,
                y: -10,
                width: 1920,
                height: 1080
            }
        );
    }

    #[test]
    fn test_parse_region_invalid_zero_dimension() {
        assert!("100,200,0,600".parse::<Region>().is_err());
        assert!("100,200,800,0".parse::<Region>().is_err());
    }

    #[test]
    fn test_parse_region_invalid_format() {
        assert!("100,200,800".parse::<Region>().is_err());
        assert!("abc,200,800,600".parse::<Region>().is_err());
    }

    #[test]
    fn test_cli_parsing_defaults() {
        let cli = Cli::try_parse_from(["rgc-cli"]).unwrap();
        assert_eq!(cli.monitor, None);
        assert_eq!(cli.region, None);
        assert_eq!(cli.output, None);
        assert!(!cli.clipboard);
        assert_eq!(cli.delay, None);
        assert_eq!(cli.format, None);
        assert!(!cli.json);
        assert!(!cli.list_monitors);
    }

    #[test]
    fn test_cli_parsing_options() {
        let cli = Cli::try_parse_from([
            "rgc-cli",
            "--monitor", "1",
            "--region", "10,20,300,400",
            "--output", "screenshot.webp",
            "-c",
            "--delay", "500",
            "--format", "webp",
            "--json",
            "-v",
        ])
        .unwrap();

        assert_eq!(cli.monitor, Some(1));
        assert_eq!(
            cli.region,
            Some(Region {
                x: 10,
                y: 20,
                width: 300,
                height: 400
            })
        );
        assert_eq!(cli.output.unwrap(), PathBuf::from("screenshot.webp"));
        assert!(cli.clipboard);
        assert_eq!(cli.delay, Some(500));
        assert_eq!(cli.format, Some(ImageFormat::Webp));
        assert!(cli.json);
        assert!(cli.verbose);
    }

    #[test]
    fn test_cli_parsing_output_file_and_dir() {
        let cli_file = Cli::try_parse_from(["rgc-cli", "--output-file", "dest.png"]).unwrap();
        assert_eq!(cli_file.output_file, Some(PathBuf::from("dest.png")));
        assert_eq!(cli_file.output, None);
        assert_eq!(cli_file.output_dir, None);

        let cli_dir = Cli::try_parse_from(["rgc-cli", "--output-dir", "captures"]).unwrap();
        assert_eq!(cli_dir.output_dir, Some(PathBuf::from("captures")));
        assert_eq!(cli_dir.output, None);
        assert_eq!(cli_file.output_file, Some(PathBuf::from("dest.png")));

        // Conflict: --output and --output-file
        assert!(Cli::try_parse_from(["rgc-cli", "-o", "out.png", "--output-file", "file.png"]).is_err());

        // Conflict: --output and --output-dir
        assert!(Cli::try_parse_from(["rgc-cli", "-o", "out.png", "--output-dir", "dir"]).is_err());

        // Conflict: --output-file and --output-dir
        assert!(Cli::try_parse_from(["rgc-cli", "--output-file", "out.png", "--output-dir", "dir"]).is_err());
    }

    #[test]
    fn test_cli_parsing_coords() {
        let cli_default = Cli::try_parse_from(["rgc-cli"]).unwrap();
        assert_eq!(cli_default.coords, CoordMode::Logical);

        let cli_physical = Cli::try_parse_from(["rgc-cli", "--coords", "physical"]).unwrap();
        assert_eq!(cli_physical.coords, CoordMode::Physical);

        let cli_logical = Cli::try_parse_from(["rgc-cli", "--coords", "logical"]).unwrap();
        assert_eq!(cli_logical.coords, CoordMode::Logical);

        assert!(Cli::try_parse_from(["rgc-cli", "--coords", "invalid"]).is_err());
    }
}
