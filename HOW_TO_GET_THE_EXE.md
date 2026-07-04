# How to get the .exe (no npm/Rust needed on your end)

This builds Frox Code into a real installer automatically, using GitHub's free build servers.
You just push code — GitHub does the compiling.

## One-time setup

1. Create a free account at https://github.com if you don't have one.
2. Create a new repository (e.g. "frox-code").
3. Upload this entire `frox-code` folder to that repository. Easiest way if you don't know git:
   on the repo page, click **"Add file" → "Upload files"**, drag the whole folder in, and commit.
4. Go to the **"Actions"** tab of your repository. You should see a workflow called
   **"Build Frox Code"** start running automatically (takes ~5-10 minutes).

## Getting the .exe

1. Once the workflow finishes (green checkmark), go to the **"Releases"** section of your repo
   (right-hand sidebar on the repo homepage, or `github.com/yourname/frox-code/releases`).
2. You'll see a draft release with build files attached — download the `.exe` (Windows installer)
   or `.AppImage`/`.deb` (Linux).
3. It's marked "draft" by default so you can review it first — click **"Publish release"** when
   you're ready to make it a real release, or just download the file directly from the draft.

## If the build fails

Click into the failed run in the Actions tab and read the red error text — it'll usually point to
a missing dependency or a typo. Paste the error back to me and I'll fix the actual code.

## After this works

Every time you push updated code to this repository, it'll automatically build a fresh `.exe` for
you — you never need to install Rust or run build commands yourself again.
