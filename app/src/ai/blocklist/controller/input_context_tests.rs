use super::*;
use crate::context_chips::GithubPullRequestChipValue;

#[test]
fn pull_request_ai_context_from_structured_prompt_chip_excludes_url() {
    let value = ChipValue::GithubPullRequest(GithubPullRequestChipValue {
        url: "https://github.com/warpdotdev/warp/pull/123".to_string(),
        number: 123,
        state: "OPEN".to_string(),
        draft: true,
        base_branch: "main".to_string(),
    });

    assert_eq!(
        pull_request_ai_context_from_chip_value(&value),
        Some(AIAgentContext::PullRequest {
            number: 123,
            state: "OPEN".to_string(),
            draft: true,
            base_branch: "main".to_string(),
        })
    );
}

#[test]
fn pull_request_ai_context_from_legacy_url_uses_metadata_defaults() {
    let value = ChipValue::Text("https://github.com/warpdotdev/warp/pull/456".to_string());

    assert_eq!(
        pull_request_ai_context_from_chip_value(&value),
        Some(AIAgentContext::PullRequest {
            number: 456,
            state: String::new(),
            draft: false,
            base_branch: String::new(),
        })
    );
}

#[test]
fn pull_request_ai_context_ignores_unparseable_chip_value() {
    let value = ChipValue::Text("not a pull request".to_string());

    assert_eq!(pull_request_ai_context_from_chip_value(&value), None);
}
