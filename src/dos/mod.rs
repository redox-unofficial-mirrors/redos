
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

use crate::{Os, OsKey, OsVideoMode};

fn rhai_err<E: ToString>(err: E) -> Box<rhai::EvalAltResult> {
    err.to_string().into()
}

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

struct File {
    id: u32,
    offset: u64,
}

struct State<
    D: Disk + 'static,
    V: Iterator<Item=OsVideoMode> + 'static,
    O: Os<D, V> + 'static,
> {
    os: O,
    fs: FileSystem<D>,
    cwd: Vec<String>,
    files: Vec<Option<File>>,
    phantom: PhantomData<V>,
}

impl<
    D: Disk + 'static,
    V: Iterator<Item=OsVideoMode> + 'static,
    O: Os<D, V> + 'static,
> State<D, V, O> {
    fn open(&mut self, path: &str) -> Result<u32, Error> {
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
        self.fs.tx(move |mut tx| {
            let node = lookup_node(&mut tx, &path_parts)?;
            Ok(node.ptr().id())
        })
    }
}

fn run_rhai<
    'i,
    D: Disk + 'static,
    V: Iterator<Item=OsVideoMode>,
    O: Os<D, V> + 'static,
    I: Iterator<Item=&'i str>,
>(
    state: &Arc<Mutex<State<D, V, O>>>,
    path_opt: Option<&str>,
    args: I
) {
    let script_opt: Option<(&str, String)> = if let Some(path) = path_opt {
        let mut path_parts = if path.starts_with("/") {
            Vec::new()
        } else {
            state.lock().cwd.clone()
        };
        for part in path.split("/").filter(|x| !x.is_empty()) {
            path_parts.push(part.to_string());
        }

        let data = match state.lock().fs.tx(|mut tx| {
            let node = lookup_node(&mut tx, &path_parts)?;
            read_node(&mut tx, &node)
        }) {
            Ok(ok) => ok,
            Err(err) => {
                println!("{}: {:?}", path, err);
                return;
            }
        };
            
        match String::from_utf8(data) {
            Ok(ok) => Some((path, ok)),
            Err(err) => {
                println!("{}: {:?}", path, err);
                return;
            }
        }
    } else {
        None
    };

    let mut engine = rhai::Engine::new();
    engine.on_print(|string| println!("{}", string));
    engine.on_debug(|string, src, pos| {
        println!("debug {:?} at {:?}: {}", src, pos, string)
    });

    {
        let state = state.clone();
        engine.register_fn("clear", move || {
            state.lock().os.clear_text();
        });
    }

    {
        let state = state.clone();
        engine.register_fn("prompt", move |prompt: &str| -> String {
            print!("{}", prompt);
            let line = read_line(&mut state.lock().os);
            for c in prompt.chars().chain(line.chars()) {
                print!("\x08 \x08");
            }
            line
        });
    }

    {
        let state = state.clone();
        engine.register_fn("cwd", move || -> String {
            let mut string = "/".to_string();
            for (i, part) in state.lock().cwd.iter().enumerate() {
                if i > 0 {
                    string.push('/');
                }
                string.push_str(part);
            }
            string
        });
    }

    {
        let state = state.clone();
        engine.register_fn("open", move |path: &str| -> Result<u32, Box<rhai::EvalAltResult>> {
            state.lock().open(path).map_err(rhai_err)
        });
    }

    {
        let state = state.clone();
        engine.register_fn("read", move |fd: u32| -> Result<String, Box<rhai::EvalAltResult>> {
            let data = state.lock().fs.tx(move |mut tx| {
                let node = tx.read_tree(TreePtr::new(fd))?;
                read_node(&mut tx, &node)
            }).map_err(rhai_err)?;
            String::from_utf8(data).map_err(rhai_err)
        });
    }

    {
        let state = state.clone();
        engine.register_fn("readdir", move |fd: u32| -> Result<rhai::Array, Box<rhai::EvalAltResult>> {
            let children = state.lock().fs.tx(move |mut tx| {
                let node = tx.read_tree(TreePtr::new(fd))?;
                readdir_node(&mut tx, &node)
            }).map_err(rhai_err)?;

            let mut array = rhai::Array::new();
            for child in children {
                if let Some(name) = child.name() {
                    array.push(name.into());
                }
            }
            Ok(array)
        });
    }

    let mut scope = rhai::Scope::new();

    let args_rhai: rhai::Array = args.map(|x| x.into()).collect();
    scope.push("args", args_rhai);

    if let Some((path, script)) = script_opt {
        match engine.run_with_scope(&mut scope, &script) {
            Ok(()) => (),
            Err(err) => {
                println!("{}: {:?}", path, err);
            }
        }
    } else {
        loop {
            print!(">> ");
            let line = read_line(&mut state.lock().os);
            println!();

            if line.as_str() == "exit" {
                break;
            }

            match engine.run_with_scope(&mut scope, &line) {
                Ok(()) => (),
                Err(err) => {
                    println!("{:?}", err);
                }
            }
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
            return run_rhai(state, path_opt, args);
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
        #[cfg(feature = "rhai")]
        _ => {
            let path = format!("/redos/{}.rhai", cmd);
            return run_rhai(state, Some(&path), args);
        },
        #[cfg(not(feature = "rhai"))]
        _ => {
            println!("{}: command not found", cmd);
        },
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
            files: Vec::new(),
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
        print!("> ");
        let line = read_line(&mut state.lock().os);
        println!();

        run(&state, &line);
    }
}
