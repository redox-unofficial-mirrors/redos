use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::marker::PhantomData;
use redoxfs::{Disk, TreePtr};
use spin::Mutex;

use crate::{Os, OsKey, OsVideoMode};
use crate::dos::{
    lookup_node,
    read_line,
    read_node,
    readdir_node,
    write_node,
    State
};

fn rhai_err<E: ToString>(err: E) -> Box<rhai::EvalAltResult> {
    err.to_string().into()
}

pub(crate) fn rhai<
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
        engine.register_fn("key", move || -> char {
            match state.lock().os.get_key() {
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
        engine.register_fn("putchar", move |c: char| {
            print!("{}", c);
        });
    }

    {
        let state = state.clone();
        engine.register_fn("highlight", move |highlight: bool| {
            state.lock().os.set_text_highlight(highlight);
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

     {
        let state = state.clone();
        engine.register_fn("write", move |fd: u32, data: &str| -> Result<(), Box<rhai::EvalAltResult>> {
            state.lock().fs.tx(move |mut tx| {
                let mut node = tx.read_tree(TreePtr::new(fd))?;
                if write_node(&mut tx, &mut node, data.as_bytes())? {
                    tx.sync_tree(node)?;
                }
                Ok(())
            }).map_err(rhai_err)
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
