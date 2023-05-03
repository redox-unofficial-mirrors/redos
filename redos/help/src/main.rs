#![no_std]

#[macro_use]
extern crate redos_abi;

use redos_abi::rt::abi;

static HELPS: &'static [(&'static str, &'static str)] = &[
    ("alias", "create and view aliases"),
    ("call", "execute a script"),
    ("cat", "print a file"),
    ("cd", "change directory"),
    ("clear", "clear screen"),
    ("date", "view system date"),
    ("df", "view available disk space"),
    ("echo", "print a string"),
    ("free", "view available memory"),
    ("history", "view command history"),
    ("ls", "list files in a directory"),
    ("mkdir", "create a directory"),
    ("rem", "leave a comment"),
    ("rm", "remove a file"),
    ("rmdir", "remove a directory"),
    ("touch", "create a file"),
    ("vi", "edit a file"),
];

#[no_mangle]
pub extern "C" fn main() {
    let args = abi().args();
    if args.is_empty() {
        for (name, help) in HELPS.iter() {
            println!("{}: {}", name, help);
        }
    } else {
        for arg in args.iter() {
            let mut found = false;
            for (name, help) in HELPS.iter() {
                if name == arg {
                    println!("{}: {}", name, help);
                    found = true;
                }
            }
            if !found {
                println!("help: {} not found", arg);
            }
        }
    }
}
