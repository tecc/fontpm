use serde::Deserialize;

// Only bare minimum
#[derive(Deserialize)]
pub struct GithubCommitData {
    pub sha: String
}
#[derive(Deserialize)]
pub struct GithubBranchData {
    pub commit: GithubCommitData
}