#![allow(dead_code)]
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct Fixture(pub PathBuf);
impl Fixture {
    pub fn new(world: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!("ano-prime-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("world.reg"), world).unwrap();
        Self(path)
    }
    pub fn run(&self, source: &str, flags: &[&str]) -> Output {
        let input = self.0.join("input.ano");
        std::fs::write(&input, format!("--! registry world.reg
{source}
")).unwrap();
        Command::new(env!("CARGO_BIN_EXE_steel")).args(flags).arg(input).output().unwrap()
    }
    pub fn accepts(&self, source: &str) -> String {
        let output = self.run(source, &["--run"]);
        assert!(output.status.success(), "{source}
{}
{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).unwrap()
    }
    pub fn refuses(&self, source: &str, cause: &str) {
        let output = self.run(source, &["--emit"]);
        assert_eq!(output.status.code(), Some(2), "{source}
{}", String::from_utf8_lossy(&output.stderr));
        assert!(String::from_utf8_lossy(&output.stderr).contains(cause), "{source}
{}", String::from_utf8_lossy(&output.stderr));
    }
}
impl Drop for Fixture {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
}
