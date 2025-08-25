# OOFs Frontend Fix Instructions

## Problem
The Vite development server is failing to start because the `@vitejs/plugin-react` package is missing from node_modules.

Error message:
```
failed to load config from D:\OOFs\Frontend\vite.config.ts
error when starting dev server:
Error [ERR_MODULE_NOT_FOUND]: Cannot find package '@vitejs/plugin-react' imported from D:\OOFs\Frontend\vite.config.ts.timestamp-1756080492455-d7fccb84df12c.mjs
```

## Solution

### Step 1: Clean the Environment
1. Delete the `node_modules` directory
2. Delete `pnpm-lock.yaml` and `package-lock.json` files if they exist

In PowerShell:
```powershell
cd Frontend
rm -rf node_modules
rm pnpm-lock.yaml package-lock.json
```

Or in Command Prompt:
```cmd
cd Frontend
rmdir /s /q node_modules
del pnpm-lock.yaml package-lock.json
```

### Step 2: Install Dependencies
Choose one of the following methods:

#### Option A: Using pnpm (recommended)
```bash
cd Frontend
pnpm install
```

#### Option B: Using npm
```bash
cd Frontend
npm install
```

### Step 3: Verify Installation
Check that `@vitejs/plugin-react` is installed:
```bash
ls node_modules/@vitejs/plugin-react
```

### Step 4: Run Development Server
```bash
cd Frontend
pnpm run dev
# or
npm run dev
```

## Troubleshooting

If the above steps don't work:

1. Make sure you have pnpm installed:
   ```bash
   npm install -g pnpm
   ```

2. Try installing the package directly:
   ```bash
   cd Frontend
   pnpm add -D @vitejs/plugin-react
   ```

3. Check your Node.js version (should be 16+):
   ```bash
   node --version
   ```

4. If you're on Windows, try using the provided batch file:
   ```bash
   cd Frontend
   .\dev-win.bat
   ```
