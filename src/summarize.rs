use crate::openai;
use anyhow::{bail, Result};

static PROMPT_TO_SUMMARIZE_DIFF: &str = r#"You are an expert programmer, and I'm looking for your assistance.
I have made some changes in the code base that I would like you to look at and tell me what's wrong with it.

Reminders about the git diff format:
For every file, there are a few metadata lines, like (for example):
```
diff --git a/lib/index.js b/lib/index.js
index aadf691..bfef603 100644
--- a/lib/index.js
+++ b/lib/index.js
```
This means that `lib/index.js` was modified in this commit. Note that this is only an example.
Then there is a specifier of the lines that were modified.
A line starting with `+` means it was added.
A line that starting with `-` means that line was deleted.
A line that starts with neither `+` nor `-` is code given for context and better understanding.
It is not part of the diff.
After the git diff of the first file, there will be an empty line, and then the git diff of the next file.

I expect you to go over every file that was changed and make sure that the changes follow good programming practices.
The output should be easily readable. When in doubt, write less comments and not more.
Do not output comments that simply repeat the contents of the file.
Write every review comment in a new line.
Comments should be in a bullet point list, each line starting with a `-`.
The review should not include comments copied from the code.
Readability is top priority. Write only the most important comments about the diff.

"#;

static PROMPT_TO_SUMMARIZE_DIFF_SUMMARIES: &str = r#"You are an expert programmer, and I'm looking for your assistance.
I have made some changes in the code base that I would like you to look at and tell me what's wrong with it.

Note that some of these files changes where too big and were omitted in the files diff presented to you.

Please review the pull request.
Write your response in bullet points, using the imperative tense following the pull request style guide.
Starting each bullet point with a `-`.
Help me find bad practices introduced in the changes and add suggestions on how this can be addressed otherwise.
Use examples to illustrate your point.
Write the most important bullet points.
The list should not be more than a few bullet points.
"#;

static PROMPT_TO_SUMMARIZE_DIFF_TITLE: &str = r#"You are an expert programmer, and I'm looking for your guidance.
I have made some changes in the code base that I would like you to review.
For some of these files changes where too big and were omitted in the files diff presented to you.
Please grade the pull request in one sentence.
The grade will act as a title for your review of the pull request.
"#;

const MAX_SUMMARY_LENGTH: usize = 3000;

pub(crate) async fn diff_summary(file_name: &str, file_diff: &str) -> Result<String> {
    debug!("summarizing file: {}", file_name);

    if file_diff.len() < MAX_SUMMARY_LENGTH {
        // filter large diffs
        let prompt = format!(
            r#"{}

THE GIT DIFF TO BE REVIEWED:
```
${}
```

THE REVIEW:
"#,
            PROMPT_TO_SUMMARIZE_DIFF, file_diff
        );

        let completion = openai::completions(&prompt).await;
        completion
    } else {
        let error_msg = format!(
            "skipping large file {}, len: {} < {}",
            file_name,
            file_diff.len(),
            MAX_SUMMARY_LENGTH
        );
        warn!("{}", error_msg);
        bail!(error_msg)
    }
}

pub(crate) async fn commit_summary(summary_points: &str) -> Result<String> {
    let prompt = format!(
        r#"{}

THE FILE SUMMARIES:
```
{}
```

Remember to write only the most important points and do not write more than a few bullet points.
THE pull request REVIEW:
"#,
        PROMPT_TO_SUMMARIZE_DIFF_SUMMARIES, summary_points
    );

    let completion = openai::completions(&prompt).await;

    completion
}

pub(crate) async fn commit_title(summary_points: &str) -> Result<String> {
    let prompt = format!(
        r#"{}

THE FILE SUMMARIES:
```
{}
```

Remember to write only one line, no more than 50 characters.
PULL REQUEST GRADE IS:
"#,
        PROMPT_TO_SUMMARIZE_DIFF_TITLE, summary_points
    );

    let completion = openai::completions(&prompt).await;

    completion
}
