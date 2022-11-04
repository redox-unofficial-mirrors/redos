use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::marker::PhantomData;
use redoxfs::{Disk, FileSystem, Node, Transaction, TreeData, TreePtr};
use spin::Mutex;
use syscall::Error;

use crate::{Os, OsKey, OsMemoryKind, OsVideoMode};
use crate::os::MemoryMapIter;

#[cfg(feature = "rhai")]
mod rhai;

mod vi;

fn lookup_node<D: Disk>(tx: &mut Transaction<D>, path: &[String]) -> Result<TreeData<Node>, Error> {
    let mut node = tx.read_tree(TreePtr::root())?;
    for part in path.iter() {
        let parent_ptr = node.ptr();
        node = tx.find_node(parent_ptr, part)?;
    }
    Ok(node)
}

fn read_node<D: Disk>(tx: &mut Transaction<D>, node: &TreeData<Node>) -> Result<Vec<u8>, Error> {
    if node.data().is_dir() {
        return Err(Error::new(syscall::EISDIR));
    }

    let size = {
        let size = node.data().size();
        if size > 1024 * 1024 {
            // Limit reading to files up to 1 MiB
            return Err(Error::new(syscall::EFBIG));
        }
        size as usize
    };

    let mut data = vec![0; size];
    let mut offset = 0;
    while offset <= data.len() {
        let count = tx.read_node_inner(&node, offset as u64, &mut data[offset..])?;
        if count == 0 {
            data.truncate(offset);
            return Ok(data);
        }
        offset += count;
    }
    Err(Error::new(syscall::EFBIG))
}

fn readdir_node<D: Disk>(tx: &mut Transaction<D>, node: &TreeData<Node>) -> Result<Vec<redoxfs::DirEntry>, Error> {
    if ! node.data().is_dir() {
        return Err(Error::new(syscall::ENOTDIR));
    }

    let mut children = Vec::new();
    tx.child_nodes(node.ptr(), &mut children)?;
    Ok(children)
}

fn write_node<D: Disk>(tx: &mut Transaction<D>, node: &mut TreeData<Node>, data: &[u8]) -> Result<bool, Error> {
    if node.data().is_dir() {
        return Err(Error::new(syscall::EISDIR));
    }

    let mut node_changed = tx.truncate_node_inner(node, 0)?;

    let mut offset = 0;
    while offset < data.len() as u64 {
        let slice = &data[offset as usize..];
        if tx.write_node_inner(node, &mut offset, slice)? {
            node_changed = true;
        }
    }

    Ok(node_changed)
}

fn read_line<
    D: Disk,
    V: Iterator<Item=OsVideoMode>
>(os: &mut dyn Os<D, V>) -> String {
    let mut line = String::new();
    loop {
        // Print cursor
        os.set_text_highlight(true);
        print!(" \x08");
        os.set_text_highlight(false);

        let key = os.get_key();

        // Wipe out cursor
        print!(" \x08");

        match key {
            OsKey::Backspace | OsKey::Delete => if ! line.is_empty() {
                print!("\x08 \x08");
                line.pop();
            },
            OsKey::Char(c) => {
                print!("{}", c);
                line.push(c)
            },
            OsKey::Enter => {
                break;
            },
            _ => (),
        }
    }
    line
}

pub(crate) struct State<
    D: Disk + 'static,
    V: Iterator<Item=OsVideoMode> + 'static,
    O: Os<D, V> + 'static,
> {
    os: O,
    fs: FileSystem<D>,
    cwd: Vec<String>,
    phantom: PhantomData<V>,
}

impl<
    D: Disk + 'static,
    V: Iterator<Item=OsVideoMode> + 'static,
    O: Os<D, V> + 'static,
> State<D, V, O> {
    fn path_parts(&self, path: &str) -> Vec<String> {
        let mut path_parts = if path.starts_with("/") {
            Vec::new()
        } else {
            self.cwd.clone()
        };
        for part in path.split("/").filter(|x| !x.is_empty()) {
            if part == "." {
                // Ignore, refers to current dir
            } else if part == ".." {
                // Parent directory
                path_parts.pop();
            } else {
                // Normal path component
                path_parts.push(part.to_string());
            }
        }
        path_parts
    }

    fn open(&mut self, path: &str) -> Result<u32, Error> {
        let path_parts = self.path_parts(path);
        self.fs.tx(move |mut tx| {
            let node = lookup_node(&mut tx, &path_parts)?;
            Ok(node.ptr().id())
        })
    }

    fn key(&mut self) -> char {
        match self.os.get_key() {
            OsKey::Left => '\u{2190}',
            OsKey::Right => '\u{2192}',
            OsKey::Up => '\u{2191}',
            OsKey::Down => '\u{2193}',
            OsKey::Backspace => '\x08',
            OsKey::Delete => '\x7F',
            OsKey::Enter => '\n',
            OsKey::Char(c) => c,
            OsKey::Other => '\0',
        }
    }
}


fn run<
    D: Disk + 'static,
    V: Iterator<Item=OsVideoMode>,
    O: Os<D, V> + 'static,
>(
    state: &Arc<Mutex<State<D, V, O>>>,
    line: &str
) {
    let mut args = line.split(' ')
        .filter(|x| !x.is_empty());
    let cmd = match args.next() {
        Some(some) => some,
        None => return,
    };

    match cmd.to_lowercase().as_str() {
        "call" => {
            let arg = match args.next() {
                Some(some) => some,
                None => ""
            };
            //TODO: support multiple args
            if args.next().is_some() {
                println!("{}: too many arguments", cmd);
                return;
            }

            let mut path = if arg.starts_with("/") {
                Vec::new()
            } else {
                state.lock().cwd.clone()
            };
            for part in arg.split("/").filter(|x| !x.is_empty()) {
                path.push(part.to_string());
            }

            let string = match state.lock().fs.tx(|mut tx| {
                let node = lookup_node(&mut tx, &path)?;
                if node.data().is_dir() {
                    return Err(Error::new(syscall::EISDIR));
                }

                read_node(&mut tx, &node)
            }) {
                Ok(data) => match String::from_utf8(data) {
                    Ok(string) => string,
                    Err(err) => {
                        println!("{}: {}: {:?}", cmd, arg, err);
                        return;
                    },
                }
                Err(err) => {
                    println!("{}: {}: {:?}", cmd, arg, err);
                    return;
                }
            };

            for line in string.lines() {
                run(state, line);
            }
        },
        "cat" => {
            for arg in args {
                let mut path = if arg.starts_with("/") {
                    Vec::new()
                } else {
                    state.lock().cwd.clone()
                };
                for part in arg.split("/").filter(|x| !x.is_empty()) {
                    path.push(part.to_string());
                }

                let string = match state.lock().fs.tx(|mut tx| {
                    let node = lookup_node(&mut tx, &path)?;
                    if node.data().is_dir() {
                        return Err(Error::new(syscall::EISDIR));
                    }

                    read_node(&mut tx, &node)
                }) {
                    Ok(data) => match String::from_utf8(data) {
                        Ok(string) => string,
                        Err(err) => {
                            println!("{}: {}: {:?}", cmd, arg, err);
                            return;
                        },
                    }
                    Err(err) => {
                        println!("{}: {}: {:?}", cmd, arg, err);
                        return;
                    }
                };
                println!("{}", string);
            }
        },
        "cd" => {
            let arg = match args.next() {
                Some(some) => some,
                None => "/"
            };
            if args.next().is_some() {
                println!("{}: too many arguments", cmd);
                return;
            }

            let mut path = if arg.starts_with("/") {
                Vec::new()
            } else {
                state.lock().cwd.clone()
            };
            for part in arg.split("/").filter(|x| !x.is_empty()) {
                path.push(part.to_string());
            }

            let mut state = state.lock();
            match state.fs.tx(|mut tx| {
                let node = lookup_node(&mut tx, &path)?;
                if ! node.data().is_dir() {
                    return Err(Error::new(syscall::ENOTDIR));
                }
                Ok(())
            }) {
                Ok(()) => {
                    state.cwd = path;
                },
                Err(err) => {
                    println!("{}: {}: {:?}", cmd, arg, err);
                }
            }
        },
        "clear" => {
            state.lock().os.clear_text();
        },
        "echo" => {
            for (i, arg) in args.enumerate() {
                if i > 0 {
                    print!(" ");
                }
                print!("{}", arg);
            }
            println!();
        },
        "ls" => {
            let mut args: Vec<_> = args.collect();
            if args.is_empty() {
                args.push(".");
            }

            let mut state = state.lock();
            for (i, arg) in args.iter().enumerate() {
                if args.len() > 1 {
                    if i > 0 {
                        println!();
                    }
                    state.os.set_text_highlight(true);
                    println!("{}:", arg);
                    state.os.set_text_highlight(false);
                }

                let path_parts = state.path_parts(arg);
                let children = match state.fs.tx(move |mut tx| {
                    let node = lookup_node(&mut tx, &path_parts)?;
                    readdir_node(&mut tx, &node)
                }) {
                    Ok(ok) => ok,
                    Err(err) => {
                        println!("{}: {}: {}", cmd, arg, err);
                        return;
                    }
                };

                for child in children {
                    if let Some(name) = child.name() {
                        println!("{}", name);
                    }
                }
            }
        },
        "mem" => {
            let print_size = |size| {
                if size >= 1 << 31 /* 2 GiB */ {
                    print!("{} GiB", size >> 30 /* 1 GiB */);
                } else if size >= 1 << 21 /* 2 MiB */ {
                    print!("{} MiB", size >> 20 /* 1 MiB */);
                } else if size >= 1 << 11 /* 2 KiB */ {
                    print!("{} KiB", size >> 10 /* 1 KiB */);
                } else {
                    print!("{} B", size);
                }
            };

            let mut free = 0;
            let mut reserved = 0;
            let mut total = 0;
            for area in state.lock().os.memory_map() {
                print!("{:08x} {:08x} ", area.base, area.base + area.size);
                print_size(area.size);
                println!(" {:?}", area.kind);

                match area.kind {
                    OsMemoryKind::Free => {
                        free += area.size;
                    },
                    _ => {
                        reserved += area.size;
                    },
                }
                total += area.size;
            }

            #[cfg(all(target_arch = "x86", target_os = "none"))]
            {
                use crate::os::ALLOCATOR;

                let stack: u32;
                unsafe { core::arch::asm!("mov {}, esp", out(reg) stack); }
                println!("Stack pointer: {:#x}", stack);

                print!("Stack used: ");
                print_size((0x70000 - stack) as u64);
                println!();

                print!("Heap free: ");
                print_size(ALLOCATOR.lock().free() as u64);
                println!();

                print!("Heap used: ");
                print_size(ALLOCATOR.lock().used() as u64);
                println!();

                print!("Heap size: ");
                print_size(ALLOCATOR.lock().size() as u64);
                println!();
            }

            print!("Free: ");
            print_size(free);
            println!();

            print!("Reserved: ");
            print_size(reserved);
            println!();

            print!("Total: ");
            print_size(total);
            println!();
        },
        "mkdir" | "md" => {
            let arg = match args.next() {
                Some(some) => some,
                None => ""
            };
            //TODO: support multiple args
            if args.next().is_some() {
                println!("{}: too many arguments", cmd);
                return;
            }

            let mut path = if arg.starts_with("/") {
                Vec::new()
            } else {
                state.lock().cwd.clone()
            };
            for part in arg.split("/").filter(|x| !x.is_empty()) {
                path.push(part.to_string());
            }
            let name = match path.pop() {
                Some(some) => some,
                None => {
                    println!("{}: no name specified", cmd);
                    return;
                }
            };

            match state.lock().fs.tx(|mut tx| {
                let parent = lookup_node(&mut tx, &path)?;
                if ! parent.data().is_dir() {
                    return Err(Error::new(syscall::ENOTDIR));
                }

                //TODO: create time!
                let ctime = (0, 0);
                tx.create_node(parent.ptr(), &name, Node::MODE_DIR, ctime.0, ctime.1)?;
                Ok(())
            }) {
                Ok(()) => (),
                Err(err) => {
                    println!("{}: {}: {:?}", cmd, arg, err);
                }
            }
        },
        #[cfg(feature = "rhai")]
        "rhai" => {
            let path_opt = args.next();
            return rhai::run(state, path_opt, args);
        },
        "rm" | "del" => {
            let arg = match args.next() {
                Some(some) => some,
                None => ""
            };
            //TODO: support multiple args
            if args.next().is_some() {
                println!("{}: too many arguments", cmd);
                return;
            }

            let mut path = if arg.starts_with("/") {
                Vec::new()
            } else {
                state.lock().cwd.clone()
            };
            for part in arg.split("/").filter(|x| !x.is_empty()) {
                path.push(part.to_string());
            }
            let name = match path.pop() {
                Some(some) => some,
                None => {
                    println!("{}: no name specified", cmd);
                    return;
                }
            };

            match state.lock().fs.tx(|mut tx| {
                let parent = lookup_node(&mut tx, &path)?;
                if ! parent.data().is_dir() {
                    return Err(Error::new(syscall::ENOTDIR));
                }

                tx.remove_node(parent.ptr(), &name, Node::MODE_FILE)?;
                Ok(())
            }) {
                Ok(()) => (),
                Err(err) => {
                    println!("{}: {}: {:?}", cmd, arg, err);
                }
            }
        },
        "rmdir" | "rd" => {
            let arg = match args.next() {
                Some(some) => some,
                None => ""
            };
            //TODO: support multiple args
            if args.next().is_some() {
                println!("{}: too many arguments", cmd);
                return;
            }

            let mut path = if arg.starts_with("/") {
                Vec::new()
            } else {
                state.lock().cwd.clone()
            };
            for part in arg.split("/").filter(|x| !x.is_empty()) {
                path.push(part.to_string());
            }
            let name = match path.pop() {
                Some(some) => some,
                None => {
                    println!("{}: no name specified", cmd);
                    return;
                }
            };

            match state.lock().fs.tx(|mut tx| {
                let parent = lookup_node(&mut tx, &path)?;
                if ! parent.data().is_dir() {
                    return Err(Error::new(syscall::ENOTDIR));
                }

                tx.remove_node(parent.ptr(), &name, Node::MODE_DIR)?;
                Ok(())
            }) {
                Ok(()) => (),
                Err(err) => {
                    println!("{}: {}: {:?}", cmd, arg, err);
                }
            }
        },
        "touch" => {
            let arg = match args.next() {
                Some(some) => some,
                None => ""
            };
            //TODO: support multiple args
            if args.next().is_some() {
                println!("{}: too many arguments", cmd);
                return;
            }

            let mut path = if arg.starts_with("/") {
                Vec::new()
            } else {
                state.lock().cwd.clone()
            };
            for part in arg.split("/").filter(|x| !x.is_empty()) {
                path.push(part.to_string());
            }
            let name = match path.pop() {
                Some(some) => some,
                None => {
                    println!("{}: no name specified", cmd);
                    return;
                }
            };

            match state.lock().fs.tx(|mut tx| {
                let parent = lookup_node(&mut tx, &path)?;
                if ! parent.data().is_dir() {
                    return Err(Error::new(syscall::ENOTDIR));
                }

                //TODO: create time!
                let ctime = (0, 0);
                tx.create_node(parent.ptr(), &name, Node::MODE_FILE, ctime.0, ctime.1)?;
                Ok(())
            }) {
                Ok(()) => (),
                Err(err) => {
                    println!("{}: {}: {:?}", cmd, arg, err);
                }
            }
        },
        "vi" => {
            match vi::vi(state, args) {
                Ok(()) => (),
                Err(err) => {
                    println!("{}: {:?}", cmd, err);
                }
            }
        }
        #[cfg(feature = "rhai")]
        _ => {
            let path = format!("/redos/{}.rhai", cmd);
            return rhai::run(state, Some(&path), args);
        },
        #[cfg(not(feature = "rhai"))]
        _ => {
            println!("{}: command not found", cmd);
        }
    }
}

pub(crate) fn main<
    D: Disk + 'static,
    V: Iterator<Item=OsVideoMode> + 'static,
    O: Os<D, V> + 'static,
>(
    os: O,
    fs: FileSystem<D>
) -> ! {
    println!("Welcome to ReDOS");

    let state = Arc::new(Mutex::new(
        State {
            os,
            fs,
            cwd: Vec::new(),
            phantom: PhantomData,
        }
    ));

    run(&state, "call redos/autoexec");

    loop {
        print!("/");
        for (i, part) in state.lock().cwd.iter().enumerate() {
            if i > 0 {
                print!("/");
            }
            print!("{}", part);
        }
        print!("# ");
        let line = read_line(&mut state.lock().os);
        println!();

        run(&state, &line);
    }
}
