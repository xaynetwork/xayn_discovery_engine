// Copyright 2022 Xayn AG
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

//! Cleans csv files of available sources.
//!
//! Required arguments:
//! 1. path to input file
//! 2. path to output file

#![forbid(unsafe_code, unsafe_op_in_unsafe_fn)]
#![deny(
    clippy::future_not_send,
    clippy::pedantic,
    noop_method_call,
    rust_2018_idioms,
    unused_qualifications
)]
#![warn(unreachable_pub, rustdoc::missing_crate_level_docs)]
#![allow(
    clippy::items_after_statements,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::must_use_candidate
)]

use std::{
    env::args,
    fs::{remove_file, File},
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
};

use anyhow::{bail, Result};
use csv::{QuoteStyle, ReaderBuilder, WriterBuilder};

fn main() -> Result<()> {
    let in_file = args().nth(1).unwrap();
    let tmp_file = Path::new(&in_file).with_extension("tmp");
    let out_file = args().nth(2).unwrap();

    fix_sources(in_file, &tmp_file)?;
    clean_sources(&tmp_file, &out_file)?;
    check_sources(out_file)?;
    remove_file(tmp_file)?;

    Ok(())
}

/// Fixes illegal linebreaks in the sources csv file.
fn fix_sources(in_file: impl AsRef<Path>, out_file: impl AsRef<Path>) -> Result<()> {
    let reader = BufReader::new(File::open(in_file)?);
    let mut writer = BufWriter::new(File::create(out_file)?);

    let mut broken_line = String::new();
    let mut count = 0;
    for line in reader.lines() {
        let line = line?;
        if !broken_line.is_empty() || line.split(';').count() != 4 {
            if broken_line.is_empty() {
                broken_line = line;
            } else {
                broken_line.push(' ');
                broken_line.push_str(&line);
            }
            if broken_line.split(';').count() == 4 {
                writer.write_all(broken_line.as_bytes())?;
                writer.write_all(&[b'\n'])?;
                broken_line.clear();
                count += 1;
            }
        } else {
            writer.write_all(line.as_bytes())?;
            writer.write_all(&[b'\n'])?;
        }
    }

    if count > 0 {
        println!("Fixed {} records with broken lines.", count);
    } else {
        println!("No broken lines to be fixed for any record.");
    }

    Ok(())
}

/// Cleans empty names in the sources csv file.
fn clean_sources(in_file: impl AsRef<Path>, out_file: impl AsRef<Path>) -> Result<()> {
    let reader = ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(false)
        .quoting(false)
        .from_path(in_file)?;
    let mut writer = WriterBuilder::new()
        .delimiter(b';')
        .has_headers(false)
        .quote_style(QuoteStyle::Never)
        .from_path(out_file)?;

    let mut count = 0;
    for record in reader.into_records() {
        let record = record?;

        let domain = if let Some(domain) = record.get(1) {
            if domain.is_empty() {
                bail!("empty domain at {:?}", record.position());
            }
            domain
        } else {
            bail!("missing domain field at {:?}", record.position());
        };
        if let Some(name) = record.get(0) {
            if name.is_empty() {
                count += 1;
            } else {
                writer.write_byte_record(&[name, domain].as_ref().into())?;
            }
        } else {
            bail!("missing name field at {:?}", record.position());
        };
    }

    if count > 0 {
        println!("Cleansed {} records with empty names.", count);
    } else {
        println!("No empty names to be cleansed for any record.");
    }

    Ok(())
}

/// Checks if the records are well formed in the sources csv file.
fn check_sources(file: impl AsRef<Path>) -> Result<()> {
    let reader = ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(false)
        .quoting(false)
        .from_path(file)?;

    for record in reader.into_records() {
        let record = record?;
        if record.len() != 2 {
            bail!("malformed source record at {:?}", record.position());
        }
        if record.get(0).unwrap().is_empty() {
            bail!("empty name field at {:?}", record.position());
        }
        if record.get(1).unwrap().is_empty() {
            bail!("empty domain field at {:?}", record.position());
        }
    }

    Ok(())
}
