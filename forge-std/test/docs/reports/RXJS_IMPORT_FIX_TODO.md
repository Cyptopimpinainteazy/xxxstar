# RxJS Import Error Fix - Task List

## Issue
Failed to compile due to incorrect import of 'map' from 'rxjs' in @polkadot/api-derive/accounts/accountId.js

## Tasks
- [x] 1. Create initial task list
- [ ] 2. Investigate the ts-sdk package structure
- [ ] 3. Check package.json for RxJS dependencies
- [ ] 4. Examine src directory for problematic archive/archive/imports
- [ ] 5. Fix any incorrect RxJS archive/archive/imports in source files
- [ ] 6. Update package.json if needed
- [ ] 7. Verify the fix compiles successfully
- [ ] 8. Test the application to ensure functionality

## Root Cause
RxJS 7+ moved operators like `map` from the main import to `'rxjs/operators'`
