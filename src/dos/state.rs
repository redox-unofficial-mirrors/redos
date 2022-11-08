use alloc::{
    boxed::Box,
    collections::{BTreeMap, VecDeque},
    string::{String, ToString},
    vec::Vec,
};
use redoxfs::{FileSystem, TreePtr};
use syscall::Error;

use crate::{Os, OsKey};
use crate::os::{OsDisk, VideoModeIter};
use super::lookup_node;

pub struct State {
    pub os: Box<dyn Os<OsDisk, VideoModeIter>>,
    pub fs: FileSystem<OsDisk>,
    pub cwd: Vec<String>,
    pub aliases: BTreeMap<String, Vec<String>>,
    pub history: VecDeque<String>,
}

impl State {
    pub fn path_parts(&self, path: &str) -> Vec<String> {
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

    pub fn open(&mut self, path: &str) -> Result<usize, Error> {
        let path_parts = self.path_parts(path);
        self.fs.tx(move |mut tx| {
            let node = lookup_node(&mut tx, &path_parts)?;
            Ok(node.ptr().id() as usize)
        })
    }

    pub fn pread(&mut self, fd: usize, buf: &mut [u8], offset: usize) -> Result<usize, Error> {
        self.fs.tx(move |tx| {
            //TODO: time
            let (atime, atime_nsec) = (0, 0);
            tx.read_node(TreePtr::new(fd as u32), offset as u64, buf, atime, atime_nsec)
        })
    }

    pub fn pwrite(&mut self, fd: usize, buf: &[u8], offset: usize) -> Result<usize, Error> {
        self.fs.tx(move |tx| {
            //TODO: time
            let (mtime, mtime_nsec) = (0, 0);
            tx.write_node(TreePtr::new(fd as u32), offset as u64, buf, mtime, mtime_nsec)
        })
    }

    pub fn truncate(&mut self, fd: usize, size: usize) -> Result<(), Error> {
        self.fs.tx(move |tx| {
            //TODO: time
            let (mtime, mtime_nsec) = (0, 0);
            tx.truncate_node(TreePtr::new(fd as u32), size as u64, mtime, mtime_nsec)
        })
    }

    pub fn key(&self) -> char {
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

    pub fn read_line(&self, prompt: &str) -> String {
        let mut history_i = self.history.len();

        let mut line = String::new();
        let mut cursor = 0;
        loop {
            // Erase current line
            while self.os.get_text_position().0 > 0 {
                print!("\x08 \x08");
            }

            // Print prompt
            print!("{}", prompt);

            // Print line, also printing cursor if inside of line
            for (i, c) in line.char_indices() {
                if i == cursor {
                    self.os.set_text_highlight(true);
                }
                print!("{}", c);
                if i == cursor {
                    self.os.set_text_highlight(false);
                }
            }

            // Print cursor if at end of line
            if cursor == line.len() {
                self.os.set_text_highlight(true);
                print!(" \x08");
                self.os.set_text_highlight(false);
            }

            let key = self.os.get_key();

            // Wipe out cursor if at end of line
            if cursor == line.len() {
                print!(" \x08");
            }

            match key {
                OsKey::Left => if cursor > 0 {
                    let mut before_opt = None;
                    for (i, _) in line.char_indices() {
                        if i < cursor {
                            before_opt = Some(i);
                        } else {
                            break;
                        }
                    }
                    if let Some(before) = before_opt {
                        cursor = before;
                    }
                },
                OsKey::Right => if cursor < line.len() {
                    let mut after_opt = None;
                    for (i, _) in line.char_indices() {
                        if i > cursor {
                            after_opt = Some(i);
                            break;
                        }
                    }
                    if let Some(after) = after_opt {
                        cursor = after;
                    } else {
                        cursor = line.len();
                    }
                },
                OsKey::Up => if history_i > 0 {
                    history_i -= 1;

                    line.clear();
                    if let Some(history) = self.history.get(history_i) {
                        line.push_str(&history);
                    }
                    cursor = line.len();
                },
                OsKey::Down => if history_i < self.history.len() {
                    history_i += 1;

                    line.clear();
                    if let Some(history) = self.history.get(history_i) {
                        line.push_str(&history)
                    }
                    cursor = line.len();
                },
                OsKey::Backspace => {
                    let mut before_opt = None;
                    for (i, _) in line.char_indices() {
                        if i < cursor {
                            before_opt = Some(i);
                        } else {
                            break;
                        }
                    }
                    if let Some(before) = before_opt {
                        let c = line.remove(before);
                        cursor -= c.len_utf8();
                    }
                },
                OsKey::Delete => if cursor < line.len() {
                    line.remove(cursor);
                },
                OsKey::Char(c) => {
                    // Ensure this does not wrap
                    //TODO: support widths other than 80?
                    if self.os.get_text_position().0 + 2 < 80 {
                        line.insert(cursor, c);
                        cursor += c.len_utf8();
                    }
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
