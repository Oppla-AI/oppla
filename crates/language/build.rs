fn main() {
    // Check for OPPLA_BUNDLE first, then fall back to ZED_BUNDLE for compatibility
    if let Ok(bundled) = std::env::var("OPPLA_BUNDLE") {
        println!("cargo:rustc-env=OPPLA_BUNDLE={}", bundled);
        println!("cargo:rustc-env=ZED_BUNDLE={}", bundled); // Keep for backward compatibility
    } else if let Ok(bundled) = std::env::var("ZED_BUNDLE") {
        println!("cargo:rustc-env=ZED_BUNDLE={}", bundled);
        println!("cargo:rustc-env=OPPLA_BUNDLE={}", bundled); // Set OPPLA_BUNDLE too
    }
}
