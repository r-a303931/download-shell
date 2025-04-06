fn main() {
    println!(
        "cargo:rustc-link-search=native={}/lib",
        std::env::var("DL_SHELL_LIBNL").unwrap()
    );
    println!("cargo:rustc-link-lib=static=nl-3");
    println!("cargo:rustc-link-lib=static=nl-route-3");
}
