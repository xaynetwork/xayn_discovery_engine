#!/usr/bin/env bash
set -euxo pipefail

# Create a temporary folder to clone the other repo
CLONE_DIR=$(mktemp -d)
DST_REPO='xaynetwork/xayn_discovery_engine_release'
EMAIL='ci@xayn.com'
USERNAME='ci'
BRANCH=${{ steps.current_branch.outputs.branch }}
SRC_COMMIT=$(git rev-parse HEAD)
SRC_COMMIT_MSG=$(git log --format=%B -n1)
git config --global user.email $EMAIL
git config --global user.name $USERNAME

# Check if the branch exists, if so, clone using the existing branch,
# if not, clone using the default branch and let git push to send to the right branch
BRANCH_EXISTS=$(git ls-remote --heads "git@github.com:$DST_REPO.git" $BRANCH | wc -l);
if [ $BRANCH_EXISTS -eq 0 ];then
  git clone --depth 1 "git@github.com:$DST_REPO.git" $CLONE_DIR
else
  git clone -b $BRANCH --depth 1 "git@github.com:$DST_REPO.git" $CLONE_DIR;
fi
cd $CLONE_DIR

# Cleaning all files on the destination repository
# --ignore-unmatch avoid to fail if the repository is empty
git rm --ignore-unmatch -r .

rsync -a --exclude example ${{ env.DART_WORKSPACE }}/ ./discovery_engine/
rsync -a --exclude example ${{ env.FLUTTER_WORKSPACE }}/ ./discovery_engine_flutter/

# Remove files from .gitignore that needs to be uploaded to the release repo
sed -i -e '/DELETE_AFTER_THIS_IN_RELEASE/,$d' ./discovery_engine/.gitignore
sed -i -e '/DELETE_AFTER_THIS_IN_RELEASE/,$d' ./discovery_engine_flutter/.gitignore

git add -A

# Commit only if something changed
if [ $(git status --porcelain | wc -l) -gt 0 ]; then
  git commit --message "This commit is a complete release.
  The next commit need to set the dependencies references."

  # change deps to the commit we just did
  sed -i s/change_me_to_commit_ref/$(git rev-parse HEAD)/ ./discovery_engine_flutter/pubspec.yaml
  git commit -a --message "$SRC_COMMIT_MSG
  https://github.com/xaynetwork/xayn_discovery_engine/commit/$SRC_COMMIT
  https://github.com/xaynetwork/xayn_discovery_engine/tree/$BRANCH"

  git push -u origin HEAD:$BRANCH
fi
