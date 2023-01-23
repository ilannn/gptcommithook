use anyhow::bail;
use anyhow::Result;
use clap::arg;
use colored::Colorize;

use clap::Args;
use tokio::try_join;

use std::collections::HashMap;

use std::fs;
use std::path::PathBuf;
use tokio::task::JoinSet;

use crate::git;
use crate::openai;
use crate::summarize;
use crate::util;

/// Splits the contents of a git diff by file.
///
/// The file path is the first string in the returned tuple, and the
/// file content is the second string in the returned tuple.
///
/// The function assumes that the file_diff input is well-formed
/// according to the Diff format described in the Git documentation:
/// https://git-scm.com/docs/git-diff
async fn process_file_diff(file_diff: &str) -> Option<(String, String)> {
    if let Some(file_name) = util::get_file_name_from_diff(file_diff) {
        let completion = summarize::diff_summary(file_name, file_diff).await;
        Some((
            file_name.to_string(),
            completion.unwrap_or_else(|_| "".to_string()),
        ))
    } else {
        None
    }
}

#[derive(Args, Debug)]
pub(crate) struct ReviewCommitChangesArgs {
    /// Debugging tool to mock git repo state
    #[arg(long)]
    git_diff_content: Option<PathBuf>,
}

pub(crate) async fn main(args: ReviewCommitChangesArgs) -> Result<()> {
    // TODO unify api key retrieval
    if let Err(err_msg) = openai::get_openai_api_key() {
        println!(
            "{}",
            r#"OPENAI_API_KEY not found in environment.
Configure the OpenAI API key with the command:

    export OPENAI_API_KEY='sk-...'
"#
            .bold()
            .yellow(),
        );
        bail!(err_msg);
    };

    println!("{}", "🤖 Asking GPT-3 to review diffs...".green().bold());

    let output = if let Some(git_diff_output) = args.git_diff_content {
        fs::read_to_string(git_diff_output)?
    } else {
        git::get_diffs()?
    };

    let file_diffs = util::split_prefix_inclusive(&output, "\ndiff --git ");

    let mut set = JoinSet::new();

    for file_diff in file_diffs {
        let file_diff = file_diff.to_owned();
        set.spawn(async move { process_file_diff(&file_diff).await });
    }

    let mut summary_for_file: HashMap<String, String> = HashMap::with_capacity(set.len());
    while let Some(res) = set.join_next().await {
        if let Some((k, v)) = res.unwrap() {
            summary_for_file.insert(k, v);
        }
    }

    let summary_points = &summary_for_file
        .iter()
        .map(|(file_name, completion)| format!("[{}]\n{}", file_name, completion))
        .collect::<Vec<String>>()
        .join("\n");

    let (title, completion) = try_join!(
        summarize::commit_title(summary_points),
        summarize::commit_summary(summary_points)
    )?;

    println!("{}", title.green().bold());
    println!("{}", completion);
    for (file_name, completion) in &summary_for_file {
        if !completion.is_empty() {
            println!("[{}]", file_name);
            println!("{}", completion);
        }
    }

    Ok(())
}
