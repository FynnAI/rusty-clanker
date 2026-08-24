#!/usr/bin/env bash
# TEST-D50: encodes "CI is the sole authority on completion" as GitHub branch
# protection's required-status-checks configuration. Run ONCE by a repository admin
# (requires `gh auth login` with repo-admin scope). This is the second and last named
# manual step in the whole verification loop, distinct from TEST-D41's EULA consent —
# deliberately not something any agent performs on its own standing authority.
set -euo pipefail

REPO="${1:-$(gh repo view --json nameWithOwner -q .nameWithOwner)}"
BRANCH="${2:-main}"

gh api --method PUT -H "Accept: application/vnd.github+json" \
  "repos/${REPO}/branches/${BRANCH}/protection" --input - <<EOF
{
  "required_status_checks": {
    "strict": true,
    "contexts": [
      "gates (ubuntu-24.04)",
      "gates (windows-2025)",
      "guardrails (ubuntu-24.04)",
      "guardrails (windows-2025)"
    ]
  },
  "enforce_admins": true,
  "required_pull_request_reviews": null,
  "restrictions": null
}
EOF

echo "Required status checks configured on ${REPO}@${BRANCH}."
