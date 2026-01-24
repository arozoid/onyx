//=== color ===//
pub const RED: &str = "\x1b[31m";
pub const BLUE: &str = "\x1b[34m";
pub const DIM: &str = "\x1b[2m";
pub const ESC: &str = "\x1b[0m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";

//=== helper funcs ===//
pub fn time_get() -> String {
    let ts = time_format::now().unwrap();
    let datetime = time_format::strftime_local("%Y-%m-%d %H:%M:%S", ts).unwrap();
    datetime
}

pub fn errln(program: &str, msg: &str) {
    let t = time_get();
    eprintln!("{RED}[{program}] err:{ESC} {msg} {DIM}[{t}]{ESC}");
}