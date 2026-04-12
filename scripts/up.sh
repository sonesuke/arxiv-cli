#!/bin/bash
set -e

docker run -d \
  --name arxiv-cli \
  -v "$(pwd):/workspaces/arxiv-cli" \
  -v "${HOME}/.config/gh:/home/user/.config/gh" \
  -e Z_AI_API_KEY="${Z_AI_API_KEY}" \
  -e CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1 \
  arxiv-cli:latest \
  sleep infinity
