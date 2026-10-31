fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set(
            "FileDescription",
            "Fast, lightweight cross-platform screen capture CLI and MCP server in pure Rust",
        );
        res.set("ProductName", "rgc-cli");
        res.set("OriginalFilename", "rgc-cli.exe");
        res.set("LegalCopyright", "Copyright (c) 2026 vusiif");
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resource: {e}");
        }
    }
}
