#!/bin/bash
set -euo pipefail

# Generate demo Git repositories for Shipyard screenshots
# Creates realistic repository history with branches, merges, tags, stashes, and various file states
# All names and content are invented - no real people or companies

DEMO_DIR="${1:-/tmp/shipyard-demo-repos}"

# Color output helpers
info() { echo "$(tput setaf 4)ℹ$(tput sgr0) $*"; }
success() { echo "$(tput setaf 2)✓$(tput sgr0) $*"; }
error() { echo "$(tput setaf 1)✗$(tput sgr0) $*" >&2; }

# Cleanup function
cleanup() {
  if [[ -d "$DEMO_DIR" ]]; then
    rm -rf "$DEMO_DIR"
    info "Cleaned up $DEMO_DIR"
  fi
}

# Trap errors and cleanup on exit
trap cleanup EXIT

info "Creating demo repositories in $DEMO_DIR"
rm -rf "$DEMO_DIR"
mkdir -p "$DEMO_DIR"

# Configuration for demo commits
FAKE_AUTHORS=(
  "Alex Rivers <alex@example.local>"
  "Jordan Lee <jordan@example.local>"
  "Casey Morgan <casey@example.local>"
  "Riley Park <riley@example.local>"
)

# Helper: Make a commit with a fake author
fake_commit() {
  local message="$1"
  local author="${FAKE_AUTHORS[$((RANDOM % ${#FAKE_AUTHORS[@]}))]}"
  local date_offset=$((RANDOM % 30 + 1)) # 1-30 days ago
  
  GIT_AUTHOR_NAME="${author%% <*}" \
  GIT_AUTHOR_EMAIL="${author#*<}" \
  GIT_AUTHOR_EMAIL="${GIT_AUTHOR_EMAIL%>}" \
  GIT_COMMITTER_NAME="${author%% <*}" \
  GIT_COMMITTER_EMAIL="${author#*<}" \
  GIT_COMMITTER_EMAIL="${GIT_COMMITTER_EMAIL%>}" \
  GIT_AUTHOR_DATE="$(date -v-${date_offset}d '+%Y-%m-%d %H:%M:%S')" \
  GIT_COMMITTER_DATE="$(date -v-${date_offset}d '+%Y-%m-%d %H:%M:%S')" \
  git commit -m "$message" --quiet
}

# ============================================================================
# Repository 1: Web Application Project
# ============================================================================

info "Creating repository 1: web-app-project"
mkdir -p "$DEMO_DIR/web-app-project"
cd "$DEMO_DIR/web-app-project"
git init --quiet

# Initial commit
cat > README.md << 'EOF'
# Web Application Project

A demo web application for testing purposes.

## Features
- User authentication
- Dashboard with widgets
- Data visualization
- Real-time updates
EOF

cat > package.json << 'EOF'
{
  "name": "web-app-project",
  "version": "1.0.0",
  "scripts": {
    "dev": "vite",
    "build": "vite build"
  }
}
EOF

mkdir -p src
cat > src/main.js << 'EOF'
import { createApp } from 'vue'
import App from './App.vue'

createApp(App).mount('#app')
EOF

cat > .gitignore << 'EOF'
node_modules/
dist/
.env
*.log
EOF

git add .
fake_commit "Initial project setup"

# Add more commits on main
for i in {1..8}; do
  echo "// Update $i" >> src/main.js
  git add src/main.js
  fake_commit "Update main application code (iteration $i)"
done

# Create feature branch
git checkout -b feature/user-dashboard --quiet
mkdir -p src/components
cat > src/components/Dashboard.vue << 'EOF'
<template>
  <div class="dashboard">
    <h1>User Dashboard</h1>
    <div class="widgets">
      <widget v-for="item in items" :key="item.id" :data="item" />
    </div>
  </div>
</template>
EOF

git add .
fake_commit "Add dashboard component"

echo "  <p>Statistics panel</p>" >> src/components/Dashboard.vue
git add .
fake_commit "Add statistics to dashboard"

# Create another feature branch
git checkout main --quiet
git checkout -b feature/auth-system --quiet

mkdir -p src/auth
cat > src/auth/login.js << 'EOF'
export function login(username, password) {
  // Authentication logic
  return authenticateUser(username, password)
}

export function logout() {
  clearSession()
}
EOF

git add .
fake_commit "Implement login system"

cat > src/auth/permissions.js << 'EOF'
export function checkPermission(user, resource) {
  return user.permissions.includes(resource)
}
EOF

git add .
fake_commit "Add permission checking"

# Merge feature branch
git checkout main --quiet
git merge feature/auth-system --no-ff --no-edit --quiet

# Create develop branch
git checkout -b develop --quiet
for i in {1..5}; do
  echo "// Develop update $i" >> src/main.js
  git add .
  fake_commit "Development iteration $i"
done

# Create tags
git checkout main --quiet
git tag -a v1.0.0 -m "Version 1.0.0 release" --quiet
git tag -a v1.1.0 -m "Version 1.1.0 with auth system" --quiet

# Create a bare remote
cd "$DEMO_DIR"
git clone --bare web-app-project web-app-project.git --quiet
cd web-app-project
git remote add origin "$DEMO_DIR/web-app-project.git"

# Make local ahead of remote
echo "// Local changes" >> src/main.js
git add .
fake_commit "Local commit not yet pushed"

# Create working tree changes
cat > src/components/NewFeature.vue << 'EOF'
<template>
  <div>New feature component</div>
</template>
EOF

echo "// Staged change" >> src/main.js
git add src/main.js

echo "// Unstaged change" >> src/auth/login.js

echo "untracked-file.txt" > untracked-file.txt

success "Repository 1 complete: web-app-project"

# ============================================================================
# Repository 2: API Server
# ============================================================================

info "Creating repository 2: api-server"
mkdir -p "$DEMO_DIR/api-server"
cd "$DEMO_DIR/api-server"
git init --quiet

cat > README.md << 'EOF'
# API Server

Backend API service for the web application.

## Endpoints
- /api/users
- /api/data
- /api/auth
EOF

mkdir -p src
cat > src/server.js << 'EOF'
const express = require('express')
const app = express()

app.listen(3000, () => {
  console.log('Server running on port 3000')
})
EOF

cat > package.json << 'EOF'
{
  "name": "api-server",
  "version": "1.0.0",
  "main": "src/server.js"
}
EOF

git add .
fake_commit "Initial API server setup"

# Add commits
for i in {1..12}; do
  echo "// Server update $i" >> src/server.js
  git add .
  fake_commit "Server enhancement $i"
done

# Create branches
git checkout -b feature/rate-limiting --quiet
mkdir -p src/middleware
cat > src/middleware/ratelimit.js << 'EOF'
module.exports = function rateLimit(options) {
  return (req, res, next) => {
    // Rate limiting logic
    next()
  }
}
EOF

git add .
fake_commit "Add rate limiting middleware"

git checkout main --quiet
git checkout -b feature/logging --quiet

cat > src/logger.js << 'EOF'
function log(level, message) {
  console.log(`[${level}] ${message}`)
}

module.exports = { log }
EOF

git add .
fake_commit "Add logging system"

git checkout main --quiet
git merge feature/logging --no-ff --no-edit --quiet

# Create hotfix branch
git checkout -b hotfix/security-patch --quiet
echo "// Security fix" >> src/server.js
git add .
fake_commit "Apply security patch"

git checkout main --quiet

# Create tags
git tag -a v2.0.0 -m "Version 2.0.0" --quiet
git tag -a v2.1.0 -m "Version 2.1.0 with logging" --quiet

# Create bare remote
cd "$DEMO_DIR"
git clone --bare api-server api-server.git --quiet
cd api-server
git remote add origin "$DEMO_DIR/api-server.git"

# Push some branches to remote
git push origin main --quiet
git push origin feature/rate-limiting --quiet

# Create stash
echo "// Work in progress" >> src/server.js
git add .
echo "// More WIP changes" >> src/logger.js
git stash push -m "WIP: Working on new feature" --quiet

# Working tree changes
mkdir -p tests
cat > tests/server.test.js << 'EOF'
describe('Server', () => {
  it('should start', () => {
    expect(true).toBe(true)
  })
})
EOF

echo "// Staged test change" >> src/server.js
git add src/server.js

echo "// Unstaged change" >> src/logger.js

echo "temp.txt" > temp.txt

success "Repository 2 complete: api-server"

# ============================================================================
# Repository 3: Documentation Site
# ============================================================================

info "Creating repository 3: docs-site"
mkdir -p "$DEMO_DIR/docs-site"
cd "$DEMO_DIR/docs-site"
git init --quiet

cat > README.md << 'EOF'
# Documentation Site

User-facing documentation for the platform.
EOF

mkdir -p docs
cat > docs/index.md << 'EOF'
# Welcome

This is the documentation home page.

## Getting Started
1. Install the software
2. Configure your environment
3. Run your first command
EOF

cat > docs/guide.md << 'EOF'
# User Guide

Learn how to use the platform effectively.
EOF

git add .
fake_commit "Initialize documentation site"

# Add commits
for i in {1..15}; do
  echo "- Section $i" >> docs/guide.md
  git add .
  fake_commit "Update documentation section $i"
done

# Create branches
git checkout -b improve/navigation --quiet
cat > docs/nav.md << 'EOF'
# Navigation

- Home
- Guide
- API Reference
EOF

git add .
fake_commit "Improve site navigation"

git checkout main --quiet

# Create tag
git tag -a v1.0 -m "First stable docs release" --quiet

# Create bare remote
cd "$DEMO_DIR"
git clone --bare docs-site docs-site.git --quiet
cd docs-site
git remote add origin "$DEMO_DIR/docs-site.git"
git push origin main --quiet

# Working tree state
echo "## New section" >> docs/guide.md
git add docs/guide.md

cat > docs/faq.md << 'EOF'
# FAQ

Common questions and answers.
EOF

success "Repository 3 complete: docs-site"

# ============================================================================
# Summary
# ============================================================================

echo ""
success "Demo repository generation complete!"
echo ""
info "Repositories created in: $DEMO_DIR"
echo ""
echo "  1. web-app-project  - 3 branches, 2 tags, staged/unstaged/untracked changes"
echo "  2. api-server       - 4 branches, 2 tags, 1 stash, mixed working tree state"
echo "  3. docs-site        - 2 branches, 1 tag, staged changes"
echo ""
info "All repositories have bare remotes for fetch/pull/push testing"
echo ""

# Don't cleanup on success - caller will use these repos
trap - EXIT
