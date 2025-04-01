# Playwright Testing for Refact

This directory contains end-to-end tests for the Refact project using Playwright. Playwright provides reliable end-to-end testing for modern web applications.

## Prerequisites

- Node.js (v14 or newer)
- npm
- A Refact API key (for certain tests)

## Setup

1. Install dependencies:

```bash
npm install
```

This will automatically install Playwright with Chromium dependencies via the postinstall script.

2. Create a `.env` file in the playwright directory with the following content:

```
REFACT_API_KEY=your_api_key_here
```

## Running Tests

### Development Mode (with UI)

To run tests with the Playwright UI, which provides a visual interface for debugging tests:

Start the lsp

```bash
cd ../refact-agent/engine \
&& cargo build \
&& target/debug/refact-lsp \
--address-url Refact \
--http-port 8001 \
--logs-stderr \
--ast \
--vecdb \
-k your-api-key \
--experimental \
-w ../../
```

Start the webserver

```bash
cd ../refact-agent/engine/gui \
npm ci \
npm run dev
```

Then in this directory.

```bash
npm start
```

This launches the Playwright UI, allowing you to:

- See test execution in real-time
- Inspect DOM elements
- Debug tests step by step
- View test traces
- Re-run specific tests

### Headless Mode

To run tests in headless mode (without browser UI), which is faster and suitable for CI/CD:

First add your api key to `refact/playwright/.env`

```
REFACT_API_KEY=your_api_key_here
```

Then run

```bash
npm run test
```

For more specific test runs, you can use the underlying Playwright command with options:

```bash
# Run a specific test file
npx playwright test tests/example.spec.ts

# Run tests with a specific tag
npx playwright test --grep "@smoke"

# Run tests in a specific project
npx playwright test --project=chromium

# Run tests with a specific reporter
npx playwright test --reporter=line
```

## Test Configuration

The Playwright configuration is defined in `playwright.config.ts` and includes:

- Test directory: `./tests`
- Browser configuration: Currently set up for Chromium
- Parallel execution settings
- Web server setup for the Refact GUI and backend

## Creating Tests

Tests are written in TypeScript using the Playwright test framework. Example:

```typescript
import { test, expect } from "@playwright/test";

test("example test", async ({ page }) => {
  await page.goto("http://localhost:5173");
  await expect(page.getByText("Welcome to Refact")).toBeVisible();
});
```

### Best Practices

1. Place test files in the `tests` directory
2. Use descriptive test names
3. Group related tests in the same file
4. Use page objects for complex pages
5. Leverage Playwright's built-in assertions

## Viewing Test Reports

After test execution, HTML reports are generated and can be viewed by:

```bash
npx playwright show-report
```

This will open the HTML report in your default browser, showing:

- Test results summary
- Test execution time
- Error details for failed tests
- Screenshots and videos (if configured)
- Traces for debugging

## Debugging Tips

1. **Use UI Mode for Interactive Debugging**:

   ```bash
   npm run start
   ```

2. **Generate and Analyze Traces**:

   ```bash
   npx playwright test --trace on
   ```

3. **Use Debug Mode**:

   ```bash
   npx playwright test --debug
   ```

4. **Record Video of Test Execution**:
   Add to config:
   ```typescript
   use: {
     video: 'on-first-retry',
   }
   ```

## CI/CD Integration

For continuous integration, set the environment variable `CI=true` to enable:

- Retry failed tests
- Reduced parallelism
- Fail if `.only` is present in tests

Example:

```bash
CI=true npm run test
```

## Advanced Configuration

See the [Playwright documentation](https://playwright.dev/docs/test-configuration) for more advanced configuration options.
