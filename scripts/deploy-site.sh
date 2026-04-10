#!/usr/bin/env bash
set -euo pipefail

# Deploy the marketing site to Cloudflare Pages
# Requires: wrangler CLI authenticated (wrangler login)

PROJECT="tetra-jig"
DIRECTORY="site"
BRANCH="${1:-main}"

echo "Deploying $DIRECTORY/ to Cloudflare Pages project '$PROJECT' (branch: $BRANCH)..."
wrangler pages deploy "$DIRECTORY/" --project-name "$PROJECT" --branch "$BRANCH" --commit-dirty=true

echo "Done. Live at https://jig.tetraresearch.io"
