use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::{
    marker::PhantomData,
    mem,
    ptr,
};
use redos_abi::Abi;
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

extern "C" fn print(s: &str) {
    print!("{}", s);
}

pub(crate) fn elf<
    'i,
    D: Disk + 'static,
    V: Iterator<Item=OsVideoMode>,
    O: Os<D, V> + 'static,
    I: Iterator<Item=&'i str>,
>(
    state: &Arc<Mutex<State<D, V, O>>>,
    path: &str,
    args: I
) {
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

    let load_addr = crate::MIBI;
    let load_size = crate::MIBI;

    if data.len() > load_size {
        println!("{}: executable too large", path);
        return;
    }

    let (entry, is64) = crate::elf_entry(&data);

    if entry < load_addr as u64 || entry > (load_addr + load_size) as u64 {
        println!("{}: entry is not valid: {:#x}", path, entry);
        return;
    }

    if is64 {
        println!("{}: 64-bit executables not supported yet", path);
        return;
    }

    let abi = Abi {
        print,
    };

    unsafe {
        ptr::write_bytes(load_addr as *mut u8, 0, load_size);
        ptr::copy(data.as_ptr(), load_addr as *mut u8, data.len());
        drop(data);

        let entry_fn: extern "C" fn(*const Abi) = mem::transmute(entry as usize);
        entry_fn(&abi);
    }
}
