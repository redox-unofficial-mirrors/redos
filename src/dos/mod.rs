use alloc::{
    boxed::Box,
    collections::BTreeMap,
    rc::Rc,
    string::{String, ToString},
    vec::Vec,
};
use core::{
    cell::RefCell,
};
use redoxfs::{Disk, FileSystem, Node, Transaction, TreeData, TreePtr};
use syscall::Error;

use crate::{Os, OsKey, OsMemoryKind};
use crate::os::{OsDisk, VideoModeIter};

mod elf;

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

pub(crate) struct State {
    os: Box<dyn Os<OsDisk, VideoModeIter>>,
    fs: FileSystem<OsDisk>,
    cwd: Vec<String>,
    aliases: BTreeMap<String, Vec<String>>,
}

impl State {
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

    fn open(&mut self, path: &str) -> Result<usize, Error> {
        let path_parts = self.path_parts(path);
        self.fs.tx(move |mut tx| {
            let node = lookup_node(&mut tx, &path_parts)?;
            Ok(node.ptr().id() as usize)
        })
    }

    fn pread(&mut self, fd: usize, buf: &mut [u8], offset: usize) -> Result<usize, Error> {
        self.fs.tx(move |tx| {
            //TODO: time
            let (atime, atime_nsec) = (0, 0);
            tx.read_node(TreePtr::new(fd as u32), offset as u64, buf, atime, atime_nsec)
        })
    }

    fn pwrite(&mut self, fd: usize, buf: &[u8], offset: usize) -> Result<usize, Error> {
        self.fs.tx(move |tx| {
            //TODO: time
            let (mtime, mtime_nsec) = (0, 0);
            tx.write_node(TreePtr::new(fd as u32), offset as u64, buf, mtime, mtime_nsec)
        })
    }

    fn truncate(&mut self, fd: usize, size: usize) -> Result<(), Error> {
        self.fs.tx(move |tx| {
            //TODO: time
            let (mtime, mtime_nsec) = (0, 0);
            tx.truncate_node(TreePtr::new(fd as u32), size as u64, mtime, mtime_nsec)
        })
    }

    fn key(&self) -> char {
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

    fn read_line(&self) -> String {
        let mut line = String::new();
        loop {
            // Print cursor
            self.os.set_text_highlight(true);
            print!(" \x08");
            self.os.set_text_highlight(false);

            let key = self.os.get_key();

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
}

fn print_size(size: u64) {
    if size >= 1 << 31 /* 2 GiB */ {
        print!("{} GiB", size >> 30 /* 1 GiB */);
    } else if size >= 1 << 21 /* 2 MiB */ {
        print!("{} MiB", size >> 20 /* 1 MiB */);
    } else if size >= 1 << 11 /* 2 KiB */ {
        print!("{} KiB", size >> 10 /* 1 KiB */);
    } else {
        print!("{} B", size);
    }
}

fn run_line(state: &Rc<RefCell<State>>, line: &str, level: usize) {
    let args = line
        .split(' ')
        .filter(|x| !x.is_empty());
    run(state, args, level);
}

fn run<'i, I: Iterator<Item=&'i str>>(
    state: &Rc<RefCell<State>>, mut args: I, level: usize
) {
    let cmd = match args.next() {
        Some(some) => some,
        None => return,
    };
    if level >= 8 {
        println!("{}: failed to resolve within recursion limit", cmd);
        return;
    }
    match cmd.to_lowercase().as_str() {
        "alias" => {
            if let Some(name) = args.next() {
                let alias: Vec<String> = args.map(|x| x.to_string()).collect();
                state.borrow_mut().aliases.insert(name.to_string(), alias);
            } else {
                for (name, alias) in state.borrow_mut().aliases.iter() {
                    print!("alias {}", name);
                    for part in alias.iter() {
                        print!(" {}", part);
                    }
                    println!();
                }
            }
        },
        "call" => {
            for arg in args {
                let string = {
                    let mut state = state.borrow_mut();

                    let path = state.path_parts(arg);

                    match state.fs.tx(|mut tx| {
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
                    }
                };

                for line in string.lines() {
                    run_line(state, line, level + 1);
                }
            }
        },
        "cat" => {
            let mut state = state.borrow_mut();
            for arg in args {
                let path = state.path_parts(arg);

                let string = match state.fs.tx(|mut tx| {
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

            let mut state = state.borrow_mut();

            let path = state.path_parts(arg);

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
            state.borrow_mut().os.clear_text();
        },
        "df" => {
            let state = state.borrow();

            let total = state.fs.header.size();
            let free = state.fs.allocator().free() * redoxfs::BLOCK_SIZE;

            print!("Free: ");
            print_size(free);
            println!();

            print!("Used: ");
            print_size(total - free);
            println!();

            print!("Total: ");
            print_size(total);
            println!();
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
        "free" => {
            let mut free = 0;
            let mut used = 0;
            let mut total = 0;
            for area in state.borrow_mut().os.memory_map() {
                match area.kind {
                    OsMemoryKind::Free => {
                        free += area.size;
                    },
                    _ => {
                        used += area.size;
                    },
                }
                total += area.size;
            }

            #[cfg(all(target_arch = "x86", target_os = "none"))]
            unsafe {
                use crate::os::HEAP;

                // Adjust values when using BIOS by heap used
                let heap_used = HEAP.used() as u64;
                free -= heap_used;
                used += heap_used;
            }

            print!("Free: ");
            print_size(free);
            println!();

            print!("Used: ");
            print_size(used);
            println!();

            print!("Total: ");
            print_size(total);
            println!();
        },
        "ls" => {
            let mut args: Vec<_> = args.collect();
            if args.is_empty() {
                args.push(".");
            }

            let mut state = state.borrow_mut();
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

                let mut names = Vec::new();
                for child in children {
                    if let Some(name) = child.name() {
                        names.push(name.to_string());
                    }
                }

                names.sort();
                for name in names {
                    println!("{}", name);
                }
            }
        },
        "mkdir" => {
            let mut state = state.borrow_mut();
            for arg in args {
                let mut path = state.path_parts(arg);
                let name = match path.pop() {
                    Some(some) => some,
                    None => {
                        println!("{}: {}: no name specified", cmd, arg);
                        return;
                    }
                };

                match state.fs.tx(|mut tx| {
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
            }
        },
        "rem" => {
            // Comment
        },
        "rm" => {
            let mut state = state.borrow_mut();
            for arg in args {
                let mut path = state.path_parts(arg);
                let name = match path.pop() {
                    Some(some) => some,
                    None => {
                        println!("{}: {}: no name specified", cmd, arg);
                        return;
                    }
                };

                match state.fs.tx(|mut tx| {
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
            }
        },
        "rmdir" => {
            let mut state = state.borrow_mut();
            for arg in args {
                let mut path = state.path_parts(arg);
                let name = match path.pop() {
                    Some(some) => some,
                    None => {
                        println!("{}: {}: no name specified", cmd, arg);
                        return;
                    }
                };

                match state.fs.tx(|mut tx| {
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
            }
        },
        #[cfg(all(target_arch = "x86", target_os = "none"))]
        "stack" => unsafe {
            let stack: u32;
            core::arch::asm!("mov {}, esp", out(reg) stack);

            let top = 0x70000;
            let bottom = 0x60000;
            let used = top - stack;
            let total = top - bottom;
            let free = total - used;

            print!("Free: ");
            print_size(free as u64);
            println!();

            print!("Used: ");
            print_size(used as u64);
            println!();

            print!("Total: ");
            print_size(total as u64);
            println!();
        },
        "touch" => {
            let mut state = state.borrow_mut();
            for arg in args {
                let mut path = state.path_parts(arg);
                let name = match path.pop() {
                    Some(some) => some,
                    None => {
                        println!("{}: {}: no name specified", cmd, arg);
                        return;
                    }
                };

                match state.fs.tx(|mut tx| {
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
            }
        },
        _ => {
            // Look for aliases
            {
                let alias_opt = state.borrow_mut().aliases.get(cmd).map(|alias| alias.clone());
                if let Some(alias) = alias_opt {
                    let mut line = String::new();
                    for part in alias {
                        if ! line.is_empty() {
                            line.push(' ');
                        }
                        line.push_str(&part);
                    }
                    for arg in args {
                        if ! line.is_empty() {
                            line.push(' ');
                        }
                        line.push_str(arg);
                    }
                    //TODO: remove recursion!
                    return run_line(state, &line, level + 1);
                }
            }

            //TODO: only run if /redos/{}.elf or /redos/{}.rhai exists
            {
                let path = format!("/redos/{}.elf", cmd);
                return elf::elf(state, &path, args);
            }

            //println!("{}: command not found", cmd);
        },
    }
}

pub(crate) fn main<
    O: Os<OsDisk, VideoModeIter> + 'static,
>(
    os: O,
    fs: FileSystem<OsDisk>
) -> ! {
    let state = Rc::new(RefCell::new(
        State {
            os: Box::new(os),
            fs,
            cwd: Vec::new(),
            aliases: BTreeMap::new(),
        }
    ));

    run_line(&state, "call redos/autoexec", 0);

    loop {
        print!("/");
        for (i, part) in state.borrow_mut().cwd.iter().enumerate() {
            if i > 0 {
                print!("/");
            }
            print!("{}", part);
        }
        print!("# ");
        let line = state.borrow_mut().read_line();
        println!();

        run_line(&state, &line, 0);
    }
}
