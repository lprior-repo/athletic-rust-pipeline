use super::super::buffer::{checked_chunk, Output};
use super::super::{converter_bytes, converter_io, ConverterObligation, CONVERTER_READ_CHUNK};
use super::combine;
use super::lifecycle::unproven;
use crate::{CrawlError, CrawlResult};
use rustix::fs::{fcntl_getfl, fcntl_setfl, OFlags};
use rustix::io::Errno;
use std::os::fd::AsFd;
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout};

pub(super) struct Pipes<'a> {
    stdin: Option<ChildStdin>,
    stdout: ChildStdout,
    input: &'a [u8],
    output: Output,
    eof: bool,
    limit: usize,
    write_error: CrawlResult<()>,
}

impl<'a> Pipes<'a> {
    pub(super) fn take(
        child: &mut Child,
        input: &'a [u8],
        limit: usize,
        command: &str,
    ) -> CrawlResult<Self> {
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            return Err(CrawlError::Invariant {
                detail: "converter did not expose its pipes".to_owned(),
            });
        };
        nonblocking(&stdin).map_err(|error| converter_io(command, error.into()))?;
        nonblocking(&stdout).map_err(|error| converter_io(command, error.into()))?;
        let mut pipes = Self {
            stdin: Some(stdin),
            stdout,
            input,
            output: Output::new(limit),
            eof: false,
            limit,
            write_error: Ok(()),
        };
        if input.is_empty() {
            pipes.close_stdin();
        }
        Ok(pipes)
    }

    pub(super) fn close_stdin(&mut self) {
        drop(self.stdin.take());
    }

    pub(super) fn tick(&mut self, command: &str) -> CrawlResult<()> {
        self.read_once(command)?;
        if let Err(error) = self.write_once(command) {
            self.write_error = Err(error);
            self.close_stdin();
        }
        Ok(())
    }

    fn read_once(&mut self, command: &str) -> CrawlResult<ReadProgress> {
        if self.eof {
            return Ok(ReadProgress::Eof);
        }
        let mut chunk = [0_u8; CONVERTER_READ_CHUNK];
        let size = self.output.read_size()?;
        let buffer = chunk
            .get_mut(..size)
            .ok_or_else(|| converter_bytes(self.limit))?;
        match rustix::io::read(&self.stdout, buffer) {
            Ok(0) => {
                self.eof = true;
                return Ok(ReadProgress::Eof);
            }
            Ok(read) => self
                .output
                .append(checked_chunk(&chunk, read)?, Path::new(command))?,
            Err(Errno::AGAIN) => return Ok(ReadProgress::Blocked),
            Err(Errno::INTR) => return Ok(ReadProgress::Interrupted),
            Err(error) => return Err(converter_io(command, error.into())),
        }
        Ok(ReadProgress::Read)
    }

    fn write_once(&mut self, command: &str) -> CrawlResult<()> {
        let Some(stdin) = &self.stdin else {
            return Ok(());
        };
        let size = self.input.len().min(CONVERTER_READ_CHUNK);
        let bytes = checked_chunk(self.input, size)?;
        match rustix::io::write(stdin, bytes) {
            Ok(0) => {
                return Err(converter_io(
                    command,
                    std::io::Error::new(
                        std::io::ErrorKind::WriteZero,
                        "converter stdin made no progress",
                    ),
                ));
            }
            Ok(written) => {
                self.input = self
                    .input
                    .get(written..)
                    .ok_or_else(|| CrawlError::Invariant {
                        detail: "converter write exceeded its supplied buffer".to_owned(),
                    })?;
            }
            Err(Errno::AGAIN | Errno::INTR) => {}
            Err(error) => return Err(converter_io(command, error.into())),
        }
        if self.input.is_empty() {
            self.close_stdin();
        }
        Ok(())
    }

    pub(super) fn finish(
        mut self,
        command: &str,
        primary: CrawlResult<()>,
    ) -> CrawlResult<Vec<u8>> {
        if let Err(error) = primary {
            return combine(Err(error), self.write_error);
        }
        let delivered = if self.input.is_empty() {
            Ok(())
        } else {
            Err(unproven(
                command,
                ConverterObligation::StdinDelivered,
                std::io::Error::other("converter exited before consuming its input"),
            ))
        };
        let drained = self.drain(command);
        combine(combine(drained, delivered), self.write_error)?;
        Ok(self.output.into_bytes())
    }

    fn drain(&mut self, command: &str) -> CrawlResult<()> {
        for _ in 0..=self.limit {
            match self.read_once(command)? {
                ReadProgress::Eof => return Ok(()),
                ReadProgress::Read | ReadProgress::Interrupted => {}
                ReadProgress::Blocked => {
                    return Err(unproven(
                        command,
                        ConverterObligation::StdoutEof,
                        std::io::Error::other("stdout remains open after group cleanup"),
                    ));
                }
            }
        }
        Err(unproven(
            command,
            ConverterObligation::StdoutEof,
            std::io::Error::other("stdout did not reach EOF within its read-work budget"),
        ))
    }
}

enum ReadProgress {
    Read,
    Eof,
    Blocked,
    Interrupted,
}

fn nonblocking(fd: impl AsFd) -> rustix::io::Result<()> {
    let flags = fcntl_getfl(&fd)?;
    fcntl_setfl(fd, flags | OFlags::NONBLOCK)
}
