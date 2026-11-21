# SignPath

The Windows code signature of each release comes from SignPath, through the
[release workflow](../.github/workflows/release.yml). See the
[code signing policy](../docs/code-signing.md).

`artifact-configurations/` holds the two configurations SignPath applies, kept
here so a change to them is reviewed like any other: SignPath reads its own
copy, so a change here is also made there, by an approver.

The workflow signs only when the repository has:

| Kind | Name | Value |
|---|---|---|
| Secret | `SIGNPATH_API_TOKEN` | The API token of a SignPath user allowed to submit signing requests. |
| Variable | `SIGNPATH_ORGANIZATION_ID` | The SignPath organisation's id. |
| Variable | `SIGNPATH_PROJECT_SLUG` | The SignPath project's slug. |
| Variable | `SIGNPATH_SIGNING_POLICY_SLUG` | `release-signing`, the policy whose requests need an approver. |

Without them, the workflow builds and publishes an installer without a
Windows code signature, as before.
