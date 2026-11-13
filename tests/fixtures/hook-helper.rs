// hook 测试辅助可执行体（由 tests/test_hooks.rs 用 rustc 现场编译；离线、跨平台）。
// 用法: hook-helper <tag> [dump路径] [log路径]
// 行为：读全部 stdin；dump 路径非空则把 stdin 原样写入；log 路径非空则追加一行 tag；
// 之后按 tag：ok=退出0；fail=退出3；panic=退出101；hang=睡眠60s（供超时测试杀掉）；
// loud=向 stdout 打印约 100KB 文本（供 stdout 纯净性 / 管道排空测试）。
use std::io::Read;
use std::io::Write;
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let tag = args.first().cloned().unwrap_or_else(|| "ok".to_string());
    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);
    if let Some(dump) = args.get(1) {
        if !dump.is_empty() {
            std::fs::write(dump, input.as_bytes()).unwrap();
        }
    }
    if let Some(log) = args.get(2) {
        if !log.is_empty() {
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log)
                .unwrap();
            let _ = writeln!(f, "{tag}");
        }
    }
    match tag.as_str() {
        "fail" => std::process::exit(3),
        "panic" => std::process::exit(101),
        "hang" => std::thread::sleep(Duration::from_secs(60)),
        "loud" => {
            let line = format!("{}\n", "x".repeat(255));
            for _ in 0..400 {
                print!("{line}");
            }
        }
        _ => {}
    }
}
