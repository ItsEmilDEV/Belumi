fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let wasm = wat::parse_file(&a[0]).expect("valid WAT");
    std::fs::write(&a[1], wasm).expect("write");
}
