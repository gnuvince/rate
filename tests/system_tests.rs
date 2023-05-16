#[test]
fn test000() {
    use std::process::Command;
    let x = Command::new(env!("CARGO_BIN_EXE_rate"))
        .arg(include_str!("test000.in"))
        .output()
        .unwrap();
    assert_eq!(x.stdout, include_bytes!("test000.out"));
}
