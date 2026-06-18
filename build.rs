fn main() {
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .file("src/wgc_capture.cpp")
        .include("src")
        .define("WIN32_LEAN_AND_MEAN", None)
        .flag("/EHsc")
        .compile("wgc_capture");
}
