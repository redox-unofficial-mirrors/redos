use alloc::{
    boxed::Box,
    string::{String, ToString},
};

use redos_abi::rt::abi;

fn rhai_err<E: ToString>(err: E) -> Box<rhai::EvalAltResult> {
    err.to_string().into()
}

pub(crate) fn rhai(args: &[&str]) {
    let script_opt: Option<(&str, String)> = if let Some(path) = args.get(0) {
        match abi().open(path).and_then(|fd| abi().read_to_string(fd)) {
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
    engine.on_debug(|string, src, pos| println!("debug {:?} at {:?}: {}", src, pos, string));

    engine.register_fn("text_clear", move || {
        abi().text_clear();
    });

    engine.register_fn("text_color", move |color: u8| {
        abi().text_color(color);
    });

    engine.register_fn("key", move || -> char { abi().key() });

    engine.register_fn("prompt", move |prompt: &str| -> String {
        print!("{}", prompt);
        let line = abi().read_line();
        for _c in prompt.chars().chain(line.chars()) {
            print!("\x08 \x08");
        }
        line
    });

    engine.register_fn("putchar", move |c: char| {
        print!("{}", c);
    });

    /*TODO: implement
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
    */

    engine.register_fn(
        "open",
        move |path: &str| -> Result<usize, Box<rhai::EvalAltResult>> {
            abi().open(path).map_err(rhai_err)
        },
    );

    engine.register_fn(
        "read",
        move |fd: usize| -> Result<String, Box<rhai::EvalAltResult>> {
            abi().read_to_string(fd).map_err(rhai_err)
        },
    );

    /*TODO: implement
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
    */

    let mut scope = rhai::Scope::new();

    let args_rhai: rhai::Array = args.iter().skip(1).map(|x| (*x).into()).collect();
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
            let line = abi().read_line();
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
