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
    slice,
    str,
};
use redos_abi::{Abi, AbiData, AbiTime, error::Error};
use redoxfs::{Disk, TreePtr};
use spin::Mutex;

use crate::{Os, OsKey, OsVideoMode};
use crate::dos::{
    lookup_node,
    read_node,
    readdir_node,
    write_node,
    State
};

unsafe fn abi_state(data: AbiData) -> &'static Arc<Mutex<State>> {
    &*(data.0 as *const Arc<Mutex<State>>)
}

extern "C" fn abi_print(_data: AbiData, ptr: *const u8, len: usize) {
    let slice = unsafe { slice::from_raw_parts(ptr, len) };
    let s = str::from_utf8(slice).unwrap();
    print!("{}", s);
}

extern "C" fn abi_text_clear(data: AbiData) {
    let state = unsafe { abi_state(data) };
    state.lock().os.clear_text();
}

extern "C" fn abi_text_color(data: AbiData, color: u8) {
    let state = unsafe { abi_state(data) };
    state.lock().os.set_text_color(color);
}

extern "C" fn abi_key(data: AbiData) -> char {
    let state = unsafe { abi_state(data) };
    state.lock().key()
}

extern "C" fn abi_time(data: AbiData, time_ptr: *mut AbiTime) {
    let state = unsafe { abi_state(data) };
    let time = unsafe { &mut *time_ptr };
    state.lock().os.time(time);
}

extern "C" fn abi_alloc(_data: AbiData, size: usize, align: usize) -> *mut u8 {
    use alloc::alloc::{alloc_zeroed, Layout};
    let layout = Layout::from_size_align(size, align).unwrap();
    unsafe { alloc_zeroed(layout) }
}

extern "C" fn abi_dealloc(_data: AbiData, ptr: *mut u8, size: usize, align: usize) {
    use alloc::alloc::{dealloc, Layout};
    let layout = Layout::from_size_align(size, align).unwrap();
    unsafe { dealloc(ptr, layout) }
}

extern "C" fn abi_open(data: AbiData, ptr: *const u8, len: usize) -> usize {
    let state = unsafe { abi_state(data) };
    let slice = unsafe { slice::from_raw_parts(ptr, len) };
    Error::mux(match str::from_utf8(slice) {
        Ok(path) => state.lock().open(path),
        Err(_err) => Err(Error::new(syscall::EINVAL))
    })
}

extern "C" fn abi_pread(data: AbiData, fd: usize, ptr: *mut u8, len: usize, offset: usize) -> usize {
    let state = unsafe { abi_state(data) };
    let slice = unsafe { slice::from_raw_parts_mut(ptr, len) };
    Error::mux(state.lock().pread(fd, slice, offset))
}

extern "C" fn abi_pwrite(data: AbiData, fd: usize, ptr: *const u8, len: usize, offset: usize) -> usize {
    let state = unsafe { abi_state(data) };
    let slice = unsafe { slice::from_raw_parts(ptr, len) };
    Error::mux(state.lock().pwrite(fd, slice, offset))
}

extern "C" fn abi_truncate(data: AbiData, fd: usize, size: usize) -> usize {
    let state = unsafe { abi_state(data) };
    Error::mux(state.lock().truncate(fd, size).and(Ok(size)))
}

pub(crate) fn elf<
    'i,
    I: Iterator<Item=&'i str>,
>(
    state: &Arc<Mutex<State>>,
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

    if cfg!(target_pointer_width = "64") {
        if !is64 {
            println!("{}: 32-bit executables not supported", path);
            return;
        }
    } else {
        if is64 {
            println!("{}: 64-bit executables not supported", path);
            return;
        }
    }

    let mut abi_args = Vec::new();
    for arg in args {
        abi_args.push((arg.as_ptr(), arg.len()));
    }
    abi_args.push((ptr::null(), 0));

    let abi = Abi {
        abi_data: AbiData(state as *const _ as usize),
        abi_args: abi_args.as_ptr(),
        abi_print,
        abi_text_clear,
        abi_text_color,
        abi_key,
        abi_time,
        abi_alloc,
        abi_dealloc,
        abi_open,
        abi_pread,
        abi_pwrite,
        abi_truncate,
    };

    unsafe {
        ptr::write_bytes(load_addr as *mut u8, 0, load_size);
        ptr::copy(data.as_ptr(), load_addr as *mut u8, data.len());
        drop(data);

        let entry_fn: extern "C" fn(*const Abi) = mem::transmute(entry as usize);
        entry_fn(&abi);
    }
}
