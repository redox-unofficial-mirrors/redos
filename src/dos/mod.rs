
use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use redoxfs::{Disk, FileSystem, Node, Transaction, TreeData, TreePtr};
use spin::Mutex;
use syscall::Error;

use crate::{Os, OsKey, OsVideoMode};

fn lookup_node<D: Disk>(tx: &mut Transaction<D>, path: &[String]) -> Result<TreeData<Node>, Error> {
    let mut node = tx.read_tree(TreePtr::root())?;
    for part in path.iter() {
        let parent_ptr = node.ptr();
        node = tx.find_node(parent_ptr, part)?;
    }
    Ok(node)
}

fn read_node<D: Disk>(tx: &mut Transaction<D>, node: &TreeData<Node>) -> Result<Vec<u8>, Error> {
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

fn read_line<
    D: Disk,
    V: Iterator<Item=OsVideoMode>
>(os: &mut dyn Os<D, V>) -> String {
    let mut line = String::new();
    loop {
        // Print cursor
        os.set_text_highlight(true);
        print!(" ");
        os.set_text_highlight(false);

        let key = os.get_key();

        // Wipe out cursor
        print!("\x08 \x08");

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

fn run_rhai<
    'i,
    D: Disk + 'static,
    V: Iterator<Item=OsVideoMode>,
    O: Os<D, V> + 'static,
    I: Iterator<Item=&'i str>,
>(
    os: &Arc<Mutex<O>>,
    fs: &Arc<Mutex<FileSystem<D>>>,
    cwd: &mut Vec<String>,
    path: &str,
    args: I
) {
    let mut path_parts = if path.starts_with("/") {
        Vec::new()
    } else {
        cwd.clone()
    };
    for part in path.split("/").filter(|x| !x.is_empty()) {
        path_parts.push(part.to_string());
    }

    let script = match fs.lock().tx(|mut tx| {
        let node = lookup_node(&mut tx, &path_parts)?;
        if node.data().is_dir() {
            return Err(Error::new(syscall::EISDIR));
        }

        read_node(&mut tx, &node)
    }) {
        Ok(ok) => match String::from_utf8(ok) {
            Ok(ok) => ok,
            Err(err) => {
                println!("{}: {:?}", path, err);
                return;
            }
        },
        Err(err) => {
            println!("{}: {:?}", path, err);
            return;
        }
    };

    let mut engine = rhai::Engine::new();
    engine.on_print(|string| println!("{}", string));
    engine.on_debug(|string, src, pos| {
        println!("debug {:?} at {:?}: {}", src, pos, string)
    });

    {
        let fs = fs.clone();
        engine.register_fn("open", move |path: &str| -> Result<u32, Box<rhai::EvalAltResult>> {
            let path_parts: Vec<String> = path
                .split("/")
                .filter(|x| !x.is_empty())
                .map(|x| x.to_string())
                .collect();
            match fs.lock().tx(move |mut tx| {
                lookup_node(&mut tx, &path_parts)
            }) {
                Ok(node) => Ok(node.ptr().id()),
                Err(err) => Err(format!("{}", err).into())
            }
        });
    }

    {
        let fs = fs.clone();
        engine.register_fn("read", move |fd: u32| -> Result<String, Box<rhai::EvalAltResult>> {
            match fs.lock().tx(move |mut tx| {
                let node = tx.read_tree(TreePtr::new(fd))?;
                read_node(&mut tx, &node)
            }) {
                Ok(data) => match String::from_utf8(data) {
                    Ok(ok) => Ok(ok),
                    Err(err) => Err(format!("{}", err).into())
                },
                Err(err) => Err(format!("{}", err).into())
            }
        });
    }

    {
        let os = os.clone();
        engine.register_fn("prompt", move |prompt: &str| -> String {
            print!("{}", prompt);
            let line = read_line(&mut *os.lock());
            for c in prompt.chars().chain(line.chars()) {
                print!("\x08 \x08");
            }
            line
        });
    }

    let mut scope = rhai::Scope::new();

    let args_rhai: rhai::Array = args.map(|x| x.into()).collect();
    scope.push("args", args_rhai);

    match engine.run_with_scope(&mut scope, &script) {
        Ok(()) => (),
        Err(err) => {
            println!("{}: {:?}", path, err);
        }
    }
}

fn run<
    D: Disk + 'static,
    V: Iterator<Item=OsVideoMode>,
    O: Os<D, V> + 'static,
>(
    os: &Arc<Mutex<O>>,
    fs: &Arc<Mutex<FileSystem<D>>>,
    cwd: &mut Vec<String>,
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
                cwd.clone()
            };
            for part in arg.split("/").filter(|x| !x.is_empty()) {
                path.push(part.to_string());
            }

            let string = match fs.lock().tx(|mut tx| {
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
                run(os, fs, cwd, line);
            }
        },
        "cat" | "type" => {
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
                cwd.clone()
            };
            for part in arg.split("/").filter(|x| !x.is_empty()) {
                path.push(part.to_string());
            }

            match fs.lock().tx(|mut tx| {
                let node = lookup_node(&mut tx, &path)?;
                if node.data().is_dir() {
                    return Err(Error::new(syscall::EISDIR));
                }

                let mut offset = 0;
                let mut block = [0; redoxfs::BLOCK_SIZE as usize];
                loop {
                    let count = tx.read_node_inner(&node, offset, &mut block)?;
                    if count == 0 {
                        break;
                    }
                    offset += count as u64;

                    for i in 0..count {
                        print!("{}", block[i] as char);
                    }
                }
                Ok(())
            }) {
                Ok(()) => (),
                Err(err) => {
                    println!("{}: {}: {:?}", cmd, arg, err);
                }
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
                cwd.clone()
            };
            for part in arg.split("/").filter(|x| !x.is_empty()) {
                path.push(part.to_string());
            }

            match fs.lock().tx(|mut tx| {
                let node = lookup_node(&mut tx, &path)?;
                if ! node.data().is_dir() {
                    return Err(Error::new(syscall::ENOTDIR));
                }
                Ok(())
            }) {
                Ok(()) => {
                    *cwd = path;
                },
                Err(err) => {
                    println!("{}: {}: {:?}", cmd, arg, err);
                }
            }
        },
        "clear" | "cls" => {
            os.lock().clear_text();
        },
        "echo" => {
            for (i, arg) in args.enumerate() {
                if i > 0 {
                    print!(" ");
                }
                print!("{}", arg);
            }
            println!()
        },
        "ls" | "dir" => {
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
                cwd.clone()
            };
            for part in arg.split("/").filter(|x| !x.is_empty()) {
                path.push(part.to_string());
            }

            match fs.lock().tx(|mut tx| {
                let node = lookup_node(&mut tx, &path)?;
                if ! node.data().is_dir() {
                    return Err(Error::new(syscall::ENOTDIR));
                }

                let mut children = Vec::new();
                tx.child_nodes(node.ptr(), &mut children)?;
                for entry in children {
                    if let Some(name) = entry.name() {
                        println!("{}", name);
                    }
                }
                Ok(())
            }) {
                Ok(()) => (),
                Err(err) => {
                    println!("{}: {}: {:?}", cmd, arg, err);
                }
            }
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
                cwd.clone()
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

            match fs.lock().tx(|mut tx| {
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
            match args.next() {
                Some(path) => {
                    run_rhai(os, fs, cwd, path, args);
                },
                //TODO: repl?
                None => {
                    println!("{}: no argument specified", cmd);
                }
            }
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
                cwd.clone()
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

            match fs.lock().tx(|mut tx| {
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
                cwd.clone()
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

            match fs.lock().tx(|mut tx| {
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
                cwd.clone()
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

            match fs.lock().tx(|mut tx| {
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
        #[cfg(feature = "rhai")]
        _ => {
            run_rhai(os, fs, cwd, &format!("/redos/{}.rhai", cmd), args);
        },
        #[cfg(not(feature = "rhai"))]
        _ => {
            println!("{}: command not found", cmd);
        },
    }
}

pub(crate) fn main<
    D: Disk + 'static,
    V: Iterator<Item=OsVideoMode>,
    O: Os<D, V> + 'static,
>(
    os: O,
    fs: FileSystem<D>
) -> ! {
    println!("Welcome to ReDOS");

    let os = Arc::new(Mutex::new(os));

    let fs = Arc::new(Mutex::new(fs));

    let mut cwd = Vec::<String>::new();

    run(&os, &fs, &mut cwd, "call redos/autoexec");

    loop {
        print!("/");
        for (i, part) in cwd.iter().enumerate() {
            if i > 0 {
                print!("/");
            }
            print!("{}", part);
        }
        print!("> ");
        let line = read_line(&mut *os.lock());
        println!();

        run(&os, &fs, &mut cwd, &line);
    }
}
