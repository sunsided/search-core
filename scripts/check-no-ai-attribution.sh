#!/bin/sh
# Refuse commit messages that carry AI attribution, co-author trailers or session links.
#
#   sh scripts/check-no-ai-attribution.sh msg <commit-msg-file>   (commit-msg stage)
#   sh scripts/check-no-ai-attribution.sh push                    (pre-push stage)
#
# The push mode reads the range from PRE_COMMIT_FROM_REF / PRE_COMMIT_TO_REF (set by prek and
# pre-commit); for a new branch it checks every commit not yet on any remote.
set -eu

pattern='co-authored-by|claude\.ai|anthropic|generated with'

case "${1:-}" in
  msg)
    if grep -qiE "$pattern" "$2"; then
      echo "commit-msg: remove Co-Authored-By trailers, session links and AI attribution." >&2
      exit 1
    fi
    ;;
  push)
    to="${PRE_COMMIT_TO_REF:-HEAD}"
    from="${PRE_COMMIT_FROM_REF:-}"
    case "$from" in
      '' | *[!0]*) [ -n "$from" ] && range="$from..$to" || range="$to --not --remotes" ;;
      *) range="$to --not --remotes" ;; # all zeros: the remote branch does not exist yet
    esac
    # shellcheck disable=SC2086 # $range holds several git arguments on purpose.
    if git log --format=%B $range | grep -qiE "$pattern"; then
      echo "pre-push: a commit message carries a co-author trailer, session link or AI attribution." >&2
      exit 1
    fi
    ;;
  *)
    echo "usage: $0 msg <file> | push" >&2
    exit 2
    ;;
esac
