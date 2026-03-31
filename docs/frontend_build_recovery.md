# AgentOS Frontend Build Recovery

This document separates project issues from local Windows environment issues when `frontend` fails to build.

## Current repo conclusion

The AgentOS frontend currently passes:

```powershell
npx tsc --noEmit
```

The remaining build failures observed in this workspace are environment-specific native binary issues, not TypeScript application errors.

## Reproducible failure sequence seen on 2026-03-31

From `frontend`:

```powershell
npm run build
```

Observed failures in this environment:

1. missing optional native package for Rollup
   - expected: `node_modules/@rollup/rollup-win32-x64-msvc`
   - local tree initially only had `@rollup/rollup-win32-x64-gnu`
   - after reinstall, the `@rollup/rollup-win32-x64-msvc` folder existed but the native file was still replaced by `rollup.win32-x64-msvc.node_policy_violated.txt`

2. quarantined esbuild executable
   - `node_modules/@esbuild/win32-x64/esbuild.exe` was missing
   - the folder contained `esbuild.exe_policy_violated.txt` instead

That means the repo code gets blocked by native dependency installation/quarantine before Vite can finish bundling.

## Files to inspect when the build fails

- `frontend/node_modules/@rollup/rollup-win32-x64-msvc`
- `frontend/node_modules/@rollup/rollup-win32-x64-msvc/rollup.win32-x64-msvc.node`
- `frontend/node_modules/@rollup/rollup-win32-x64-msvc/rollup.win32-x64-msvc.node_policy_violated.txt`
- `frontend/node_modules/@rollup/rollup-win32-x64-gnu`
- `frontend/node_modules/@esbuild/win32-x64/esbuild.exe`
- `frontend/node_modules/@esbuild/win32-x64/esbuild.exe_policy_violated.txt`

## Recovery procedure on Windows

Run from `frontend`:

1. remove the local dependency tree

```powershell
Remove-Item -Recurse -Force node_modules
```

2. reinstall from the lockfile

```powershell
npm ci
```

3. confirm the Windows-native binaries exist

```powershell
Test-Path node_modules/@rollup/rollup-win32-x64-msvc
Test-Path node_modules/@esbuild/win32-x64/esbuild.exe
```

4. run the validations again

```powershell
npx tsc --noEmit
npm run build
```

## If esbuild is quarantined again

If either of these appears again, the blocker is external to the repo:

- `rollup.win32-x64-msvc.node_policy_violated.txt`
- `esbuild.exe_policy_violated.txt`

Required remediation:

1. restore or whitelist the blocked executable with the machine's security tooling
2. reinstall dependencies after the policy exception is in place
3. rerun `npm ci`
4. rerun `npm run build`

## Clean-machine validation checklist

On a clean Windows machine, C8 can be closed if all of these pass:

1. `npm ci`
2. `npx tsc --noEmit`
3. `npm run build`
4. the built app still shows:
   - real version
   - real plan
   - real updater state
   - real integrations
   - partial capabilities marked as partial

## What this means for C8

Current repo status:

- frontend wiring is honest
- TypeScript checks pass
- build remains blocked only when the local environment corrupts or quarantines native dependencies

So C8 stays partial in this workspace, but the repo is prepared for closure on a clean Windows machine.
