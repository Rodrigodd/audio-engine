fn main() {
    let target = std::env::var("TARGET").unwrap();
    if target.contains("android") {
        println!("cargo:rustc-link-lib=c++_shared");
    }
}
