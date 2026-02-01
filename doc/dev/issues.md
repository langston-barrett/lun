# Issues

We use [GitHub issues] to track bugs, feature ideas, and host discussions.

[GitHub issues]: https://github.com/langston-barrett/lun/issues

## Labels

<!-- See also scripts/labels.sh -->

We use issue labels to organize our issues and PRs. This requires a modicum of
effort, but can make it substantially easier to find something you're looking
for down the line.

Our labels form a filesystem-like hierarchy, with `/` used as a separator. The
top-level "directories" are:

- `area/`: The part(s) of the codebase and/or domain of interest that are
  relevant to this issue/PR.
<!-- - `size/`: How big and/or difficult it would be to fix this issue. -->
- `status/`: Where this issue/PR is in its lifecycle (see below).
- `topic/`: The sort of improvement this issue/PR targets. Examples include
  `topic/performance`, `topic/tech debt`, `topic/ux`.
- `type/`: Whether this is a bug report, feature request, refactoring idea,
  or question.

You can find a comprehensive list of labels in [the GitHub label UI]. Almost all
of them have individual descriptions.

[the GitHub label UI]: https://github.com/langston-barrett/lun/labels

`type/` labels are mutually exclusive. `topic/` labels are usually but not
necessarily mutually exclusive.

## `status/`

The `status/` labels describe where an issue is in its lifecycle. They mostly
apply to bugs. The notional workflow is a progression through the following
stages, though several might be skipped along the way:

- `status/needs repro`: The issue does not have sufficient information to
  reproduce the reported bug.
- `status/needs mcve`: The issue has sufficient information to reproduce
  the reported bug, but it is not sufficiently simple (i.e., not a minimal,
  complete, and verified example).
- `status/needs test`: The issue has an MCVE, but lacks an in-repo
  expected-failure test tracking the completion of the ticket.
- `status/has test`: The issue has test in the repo.
