use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, bail};
use clap::ValueEnum;

use crate::ui::DisplaySize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum FileListDelimiter {
    #[default]
    Newline,
    Nul,
    Tab,
    Pipe,
}

impl FileListDelimiter {
    fn byte(self) -> u8 {
        match self {
            Self::Newline => b'\n',
            Self::Nul => 0,
            Self::Tab => b'\t',
            Self::Pipe => b'|',
        }
    }
}

#[derive(Default)]
pub struct DryRunPlan {
    paths: Vec<PathBuf>,
    size: u128,
}

impl DryRunPlan {
    pub fn record(&mut self, path: PathBuf, size: u128) {
        // Scanner roots are absolute, but keep the boundary honest for other callers.
        assert!(path.is_absolute(), "dry-run paths must be absolute");
        if !self.paths.contains(&path) {
            self.paths.push(path);
            self.size += size;
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "dry run: would have cleaned {} across {} items",
            DisplaySize(self.size as f64),
            self.paths.len()
        )
    }

    fn write_list(&self, writer: &mut impl Write, delimiter: FileListDelimiter) -> io::Result<()> {
        let mut paths: Vec<_> = self.paths.iter().collect();
        paths.sort_by(|a, b| {
            a.as_os_str()
                .as_encoded_bytes()
                .cmp(b.as_os_str().as_encoded_bytes())
        });
        for path in paths {
            writer.write_all(path.as_os_str().as_encoded_bytes())?;
            writer.write_all(&[delimiter.byte()])?;
        }
        Ok(())
    }

    pub fn finish(&self, delimiter: FileListDelimiter) -> anyhow::Result<Option<PathBuf>> {
        if self.paths.is_empty() {
            return Ok(None);
        }
        // mktemp creates a private file atomically; it stays available after jett exits.
        let output = Command::new("mktemp")
            .args(["-t", "jett-dry-run.XXXXXXXXXX"])
            .output()
            .context("could not create dry-run list with mktemp")?;
        if !output.status.success() {
            bail!("mktemp failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        let mut name = output.stdout;
        if name.last() == Some(&b'\n') {
            name.pop();
        }
        #[cfg(unix)]
        let name = {
            use std::os::unix::ffi::OsStringExt;
            OsString::from_vec(name)
        };
        #[cfg(not(unix))]
        let name = OsString::from(String::from_utf8(name).context("invalid mktemp path")?);
        let path = PathBuf::from(name);
        let result = (|| {
            let mut file = OpenOptions::new().write(true).truncate(true).open(&path)?;
            self.write_list(&mut file, delimiter)?;
            file.flush()
        })();
        if let Err(error) = result {
            let _ = fs::remove_file(&path);
            return Err(error).context("could not write dry-run list");
        }
        Ok(Some(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_is_sorted_deduplicated_absolute_and_counts_folders_once() {
        let mut plan = DryRunPlan::default();
        for (path, size) in [("/tmp/z", 1024), ("/tmp/a-folder", 2048), ("/tmp/z", 1024)] {
            plan.record(PathBuf::from(path), size);
        }
        assert_eq!(
            plan.summary(),
            "dry run: would have cleaned 3.0K across 2 items"
        );
        for delimiter in [
            FileListDelimiter::Newline,
            FileListDelimiter::Nul,
            FileListDelimiter::Tab,
            FileListDelimiter::Pipe,
        ] {
            let path = plan.finish(delimiter).unwrap().unwrap();
            let content = fs::read(&path).unwrap();
            let mut expected = b"/tmp/a-folder".to_vec();
            expected.push(delimiter.byte());
            expected.extend_from_slice(b"/tmp/z");
            expected.push(delimiter.byte());
            assert_eq!(content, expected);
            assert_eq!(
                content
                    .split(|b| *b == delimiter.byte())
                    .filter(|p| !p.is_empty())
                    .count(),
                plan.paths.len()
            );
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                assert_eq!(
                    fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                    0o600
                );
            }
            fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn zero_deletions_have_summary_but_no_list() {
        let plan = DryRunPlan::default();
        assert_eq!(
            plan.summary(),
            "dry run: would have cleaned 0 across 0 items"
        );
        assert_eq!(plan.finish(FileListDelimiter::Newline).unwrap(), None);
    }

    #[cfg(unix)]
    #[test]
    fn nul_list_preserves_hostile_and_non_utf8_filenames_and_byte_order() {
        use std::os::unix::ffi::OsStringExt;
        let names = [
            b"/tmp/a\n\t|\xff".to_vec(),
            b"/tmp/a-".to_vec(),
            b"/tmp/a/child".to_vec(),
        ];
        let mut plan = DryRunPlan::default();
        for name in &names {
            plan.record(PathBuf::from(OsString::from_vec(name.clone())), 1);
        }
        let mut list = Vec::new();
        plan.write_list(&mut list, FileListDelimiter::Nul).unwrap();
        let mut expected = names.to_vec();
        expected.sort();
        let mut bytes = Vec::new();
        for name in expected {
            bytes.extend(name);
            bytes.push(0);
        }
        assert_eq!(list, bytes);
    }
}
