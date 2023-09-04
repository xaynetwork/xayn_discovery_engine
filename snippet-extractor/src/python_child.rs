// Copyright 2023 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::{
    io::{self, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use rmp_serde::{
    config::{DefaultConfig, StructMapConfig},
    decode::ReadReader,
};
use serde::{de::DeserializeOwned, Serialize};

pub(crate) struct PythonChild {
    child: Child,
    // Hint: We always write the whole package at once, so no point in using a BufWriter
    write_to: Option<rmp_serde::Serializer<ChildStdin, StructMapConfig<DefaultConfig>>>,
    read_from: Option<rmp_serde::Deserializer<ReadReader<BufReader<ChildStdout>>, DefaultConfig>>,
}

impl PythonChild {
    pub(crate) fn spawn(
        workspace: impl AsRef<Path>,
        python_file: impl AsRef<Path>,
    ) -> Result<Self, io::Error> {
        let mut child = Command::new("pipenv")
            .args(["run", "python"])
            .arg(python_file.as_ref())
            .current_dir(workspace.as_ref())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let write_to = child.stdin.take().unwrap(/* Command.stdin(piped) was used */);
        let write_to = rmp_serde::Serializer::new(write_to).with_struct_map();
        let read_from = child.stdout.take().unwrap(/* Command.stdout(piped) was used */);
        let read_from = rmp_serde::Deserializer::new(BufReader::new(read_from));
        Ok(PythonChild {
            child,
            write_to: Some(write_to),
            read_from: Some(read_from),
        })
    }

    pub(crate) fn read_message<V, E>(&mut self) -> Result<V, E>
    where
        V: DeserializeOwned,
        E: From<rmp_serde::decode::Error>,
    {
        let deserializer = self.read_from.as_mut().unwrap(/* only None in Drop */);
        V::deserialize(deserializer).map_err(E::from)
    }

    pub(crate) fn write_message<M, E>(&mut self, msg: &M) -> Result<(), E>
    where
        M: Serialize,
        E: From<rmp_serde::encode::Error> + From<io::Error>,
    {
        let serializer = self.write_to.as_mut().unwrap(/* only None in Drop */);
        msg.serialize(&mut *serializer)?;
        serializer.get_mut().flush()?;
        Ok(())
    }

    pub(crate) fn send_command<C, M, E>(&mut self, cmd: &C, map_err: M) -> Result<C::Value, E>
    where
        C: PipeCommand,
        M: Fn(String) -> E,
        E: From<rmp_serde::encode::Error> + From<rmp_serde::decode::Error> + From<io::Error>,
    {
        self.write_message::<_, E>(&Message { tag: C::TAG, cmd })?;
        self.read_message::<Result<C::Value, String>, E>()?
            .map_err(map_err)
    }
}

impl Drop for PythonChild {
    fn drop(&mut self) {
        // drop/close pipes first which will trigger the childs shutdown
        drop(self.write_to.take());
        drop(self.read_from.take());

        let start = Instant::now();
        while self.child.try_wait().is_ok_and(|exited| exited.is_none()) {
            if Instant::now().duration_since(start) > Duration::from_millis(150) {
                self.child.kill().ok();
            }
            thread::yield_now();
        }
    }
}

pub(crate) trait PipeCommand: Serialize {
    type Value: DeserializeOwned;
    const TAG: &'static str;
}

#[derive(Serialize)]
struct Message<'a, T: PipeCommand> {
    tag: &'a str,
    cmd: &'a T,
}
