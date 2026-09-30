//! The v1 corpus ratchet (docs/TESTING.md): one trial per archive in tests/corpus/v1, each
//! checked against its line in tests/corpus/status.txt. `CUECUE_BLESS=1` rewrites that file.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

use libtest_mimic::{Arguments, Failed, Trial};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Pass,
    Fail,
    Skip,
}

impl Status {
    fn parse(s: &str) -> Option<Status> {
        match s {
            "pass" => Some(Status::Pass),
            "fail" => Some(Status::Fail),
            "skip" => Some(Status::Skip),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Status::Pass => "pass",
            Status::Fail => "fail",
            Status::Skip => "skip",
        }
    }
}

/// A line of status.txt: `<status> <path>  # <reason>`.
#[derive(Debug, Clone)]
struct Entry {
    status: Status,
    reason: Option<String>,
}

struct Case {
    /// Path relative to tests/corpus/v1, also the trial name.
    name: String,
    /// The archive's sections in order: (name, content).
    sections: Vec<(String, String)>,
}

impl Case {
    fn section(&self, name: &str) -> Option<&str> {
        self.sections
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, body)| body.as_str())
    }

    fn skip_reason(&self) -> Option<String> {
        self.section("oracle/skip").map(|s| s.trim().to_owned())
    }

    /// Evaluates the archive and compares the result with the oracle's.
    fn run(&self) -> Result<(), String> {
        let want = self
            .section("oracle/export.json")
            .or_else(|| self.section("oracle/error"))
            .ok_or_else(|| "archive has no oracle section".to_owned())?;
        let files: Vec<(&str, &str)> = self
            .sections
            .iter()
            .filter(|(name, _)| name.ends_with(".cue"))
            .map(|(name, body)| (name.as_str(), body.as_str()))
            .collect();
        let got = match cuecue_eval::export_json(&files) {
            Ok(json) => json,
            Err(errors) => normalize(&errors),
        };
        if want == got {
            Ok(())
        } else {
            Err(format!("want:\n{want}got:\n{got}"))
        }
    }

    fn status(&self) -> Status {
        match (self.skip_reason(), self.run()) {
            (Some(_), _) => Status::Skip,
            (None, Ok(())) => Status::Pass,
            (None, Err(_)) => Status::Fail,
        }
    }
}

/// Errors as the oracle prints them: `<kind> <path>` lines, sorted and deduplicated.
fn normalize(errors: &[cuecue_eval::Error]) -> String {
    let lines: BTreeSet<String> = errors
        .iter()
        .map(|e| format!("{} {}", e.kind.as_str(), e.path.as_deref().unwrap_or("-")))
        .collect();
    lines.into_iter().map(|l| l + "\n").collect()
}

/// Splits a txtar archive into its sections, dropping the leading comment.
fn parse_txtar(text: &str) -> Vec<(String, String)> {
    let mut sections: Vec<(String, String)> = Vec::new();
    for line in text.split_inclusive('\n') {
        let header = line
            .trim_end_matches('\n')
            .strip_prefix("-- ")
            .and_then(|l| l.strip_suffix(" --"));
        match (header, sections.last_mut()) {
            (Some(name), _) => sections.push((name.trim().to_owned(), String::new())),
            (None, Some((_, body))) => body.push_str(line),
            (None, None) => {}
        }
    }
    sections
}

fn load_cases(dir: &Path) -> Vec<Case> {
    let mut paths = Vec::new();
    collect_archives(dir, &mut paths);
    paths.sort();
    paths
        .into_iter()
        .map(|path| Case {
            name: path
                .strip_prefix(dir)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/"),
            sections: parse_txtar(&fs::read_to_string(&path).unwrap()),
        })
        .collect()
}

fn collect_archives(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_archives(&path, out);
        } else if path.extension().is_some_and(|e| e == "txtar") {
            out.push(path);
        }
    }
}

fn read_status(path: &Path) -> BTreeMap<String, Entry> {
    let Ok(text) = fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let (line, reason) = match line.split_once(" # ") {
                Some((l, r)) => (l, Some(r.trim().to_owned())),
                None => (line, None),
            };
            let (status, name) = line.trim().split_once(' ').expect("`<status> <path>`");
            let status = Status::parse(status).expect("status is pass, fail or skip");
            (name.trim().to_owned(), Entry { status, reason })
        })
        .collect()
}

fn bless(cases: &[Case], old: &BTreeMap<String, Entry>, path: &Path) {
    let mut out = String::new();
    for case in cases {
        let status = case.status();
        let reason = match status {
            Status::Pass => None,
            Status::Skip => case.skip_reason(),
            // Keep a hand-written reason while the case still fails.
            Status::Fail => old
                .get(&case.name)
                .filter(|e| e.status == Status::Fail)
                .and_then(|e| e.reason.clone()),
        };
        match reason {
            Some(r) => writeln!(out, "{} {}  # {r}", status.as_str(), case.name),
            None => writeln!(out, "{} {}", status.as_str(), case.name),
        }
        .unwrap();
    }
    fs::write(path, out).unwrap();
}

fn main() {
    let args = Arguments::from_args();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus");
    let status_path = root.join("status.txt");
    let cases = load_cases(&root.join("v1"));

    if env::var_os("CUECUE_BLESS").is_some() {
        bless(&cases, &read_status(&status_path), &status_path);
    }
    let expected = read_status(&status_path);

    // status.txt must list exactly the archives, with `skip` exactly where the oracle skipped.
    let listed: BTreeSet<String> = expected.keys().cloned().collect();
    let skips: BTreeMap<String, bool> = cases
        .iter()
        .map(|c| (c.name.clone(), c.skip_reason().is_some()))
        .collect();
    let coverage = {
        let expected = expected.clone();
        Trial::test("status.txt matches the corpus", move || {
            let archives: BTreeSet<String> = skips.keys().cloned().collect();
            let missing: Vec<_> = archives.difference(&listed).collect();
            let stale: Vec<_> = listed.difference(&archives).collect();
            let wrong_skip: Vec<_> = skips
                .iter()
                .filter(|(name, skip)| {
                    expected
                        .get(*name)
                        .is_some_and(|e| (e.status == Status::Skip) != **skip)
                })
                .map(|(name, _)| name)
                .collect();
            if missing.is_empty() && stale.is_empty() && wrong_skip.is_empty() {
                Ok(())
            } else {
                Err(Failed::from(format!(
                    "missing: {missing:?}\nstale: {stale:?}\nwrong skip status: {wrong_skip:?}\n\
                     run `CUECUE_BLESS=1 cargo test -p cuecue-eval --test corpus`"
                )))
            }
        })
    };

    let mut trials = vec![coverage];
    for case in cases {
        let entry = expected.get(&case.name).cloned();
        let skip = case.skip_reason().is_some();
        trials.push(
            Trial::test(case.name.clone(), move || {
                let Some(entry) = entry else {
                    return Err("not in status.txt; bless it".into());
                };
                match (entry.status, case.run()) {
                    (Status::Pass, Ok(())) | (Status::Fail, Err(_)) => Ok(()),
                    (Status::Pass, Err(diff)) => Err(format!("regression:\n{diff}").into()),
                    (Status::Fail, Ok(())) => {
                        Err("passes now; bless status.txt to record it".into())
                    }
                    (Status::Skip, _) => Err("listed as skip without an oracle/skip".into()),
                }
            })
            .with_ignored_flag(skip),
        );
    }
    libtest_mimic::run(&args, trials).exit();
}
