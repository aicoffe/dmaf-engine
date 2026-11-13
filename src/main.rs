//! dmaf-engine 二进制入口：解析参数 → 执行 → 输出 → 退出码。

use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = dmaf_engine::run(&args);
    if !out.stdout.is_empty() {
        print!("{}", out.stdout);
        let _ = std::io::stdout().flush();
    }
    if !out.stderr.is_empty() {
        eprint!("{}", out.stderr);
        let _ = std::io::stderr().flush();
    }
    std::process::exit(out.code);
}
