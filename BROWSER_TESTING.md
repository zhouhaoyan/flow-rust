# 浏览器自动化测试指南

## 概述

浏览器自动化测试用于验证Web应用的用户界面和交互功能。对于Flower Rust项目，这包括：
- 用户登录流程
- 自然语言记录提交
- 记录确认工作流
- 管理面板数据表操作
- 前端与后端API集成

## 推荐工具

### Playwright（推荐）
- 微软开发的现代浏览器自动化框架
- 支持Chromium、Firefox、WebKit
- 自动等待元素，减少flaky测试
- 内置截图、视频录制、网络拦截

### 优势
- 简单易用的API
- 跨浏览器测试
- 良好的调试工具
- 与CI/CD集成

## 安装与设置

### 1. 安装Node.js和npm
确保系统已安装Node.js 16+：
```bash
node --version
npm --version
```

### 2. 初始化npm项目（如果尚未）
```bash
npm init -y
```

### 3. 安装Playwright
```bash
npm install --save-dev @playwright/test
```

### 4. 安装浏览器
```bash
npx playwright install chrome
# 或安装所有浏览器
npx playwright install
```

### 5. 配置Playwright
创建 `playwright.config.js` 文件（已提供）：
```javascript
import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './browser_tests',
  use: {
    baseURL: 'http://localhost:3000',
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  webServer: {
    command: 'cargo run',
    url: 'http://localhost:3000/health',
    reuseExistingServer: true,
  },
});
```

## 编写测试

### 测试结构
```javascript
import { test, expect } from '@playwright/test';

test.describe('功能描述', () => {
  test('测试用例描述', async ({ page }) => {
    // 测试步骤
    await page.goto('/path');
    await expect(page.locator('selector')).toBeVisible();
  });
});
```

### 常用API
- `page.goto(url)` - 导航到页面
- `page.fill(selector, text)` - 填写表单
- `page.click(selector)` - 点击元素
- `page.waitForResponse(url)` - 等待API响应
- `expect(locator).toBeVisible()` - 断言元素可见
- `expect(locator).toHaveText(text)` - 断言文本内容

### 示例测试用例

本项目已提供 `browser_tests/basic_workflow.spec.js` 包含以下测试：

1. **访问登录页面** - 验证登录页面元素
2. **成功登录** - 测试认证流程
3. **提交自然语言记录** - 测试主要功能
4. **查看记录列表** - 验证数据展示
5. **访问管理面板** - 测试管理界面
6. **查看品种档案数据** - 验证API集成

## 运行测试

### 启动后端服务器
```bash
cargo run
```

### 运行浏览器测试
```bash
# 运行所有浏览器测试
npm run test:browser

# 运行UI模式（可视化）
npm run test:browser:ui

# 调试模式
npm run test:browser:debug

# 安装浏览器（首次运行前）
npm run install:browsers
```

### 在CI/CD中运行
```bash
# 安装依赖
npm ci
# 安装浏览器
npx playwright install --with-deps
# 运行测试
npx playwright test
```

## 测试最佳实践

### 1. 使用测试ID
为重要元素添加 `data-testid` 属性：
```html
<button data-testid="submit-btn">提交</button>
```

测试中使用：
```javascript
await page.click('[data-testid="submit-btn"]');
```

### 2. 页面对象模式
创建可重用的页面对象类：
```javascript
// pages/LoginPage.js
export class LoginPage {
  constructor(page) {
    this.page = page;
    this.usernameInput = page.locator('[data-testid="username"]');
    this.passwordInput = page.locator('[data-testid="password"]');
    this.submitButton = page.locator('[data-testid="submit"]');
  }
  
  async login(username, password) {
    await this.usernameInput.fill(username);
    await this.passwordInput.fill(password);
    await this.submitButton.click();
  }
}
```

### 3. 测试隔离
每个测试应独立，不依赖其他测试状态：
- 使用独立测试数据
- 清理测试后状态
- 避免共享用户会话

### 4. 等待策略
使用Playwright的自动等待，避免硬编码sleep：
```javascript
// 正确
await page.locator('.modal').waitFor({ state: 'visible' });

// 避免
await page.waitForTimeout(5000); // 硬编码等待
```

## 调试技巧

### 1. 录制测试
使用Playwright Codegen录制用户操作：
```bash
npx playwright codegen http://localhost:3000
```

### 2. 查看追踪
测试失败时查看追踪：
```bash
npx playwright show-trace trace.zip
```

### 3. 截图和视频
配置文件中启用：
```javascript
use: {
  screenshot: 'only-on-failure',
  video: 'retain-on-failure',
}
```

### 4. 网络拦截
模拟API响应或验证请求：
```javascript
await page.route('**/api/record', route => {
  console.log('请求URL:', route.request().url());
  route.continue();
});
```

## 常见问题

### 1. 元素找不到
- 确认选择器正确
- 检查元素是否在iframe中
- 验证页面是否完全加载
- 使用 `page.waitForSelector()` 

### 2. 测试不稳定
- 避免硬编码等待
- 使用更稳定的选择器
- 增加重试机制
- 检查网络请求

### 3. 跨浏览器问题
- 在所有目标浏览器上测试
- 注意CSS和JavaScript差异
- 处理浏览器特有行为

### 4. 认证状态
- 每个测试独立登录
- 使用测试专用账户
- 清理cookies和localStorage

## 集成到开发流程

### 1. 预提交钩子
在提交前运行关键测试：
```bash
# .husky/pre-commit
npm run test:browser -- --grep "关键路径"
```

### 2. CI/CD流水线
在CI中运行完整测试套件：
```yaml
# .github/workflows/test.yml
jobs:
  browser-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions/setup-node@v2
      - run: npm ci
      - run: npx playwright install --with-deps
      - run: npx playwright test
```

### 3. 测试报告
生成HTML报告：
```bash
npx playwright test --reporter=html
# 查看报告
npx playwright show-report
```

## 扩展测试覆盖

### 1. 性能测试
```javascript
test('页面加载性能', async ({ page }) => {
  const startTime = Date.now();
  await page.goto('/');
  const loadTime = Date.now() - startTime;
  expect(loadTime).toBeLessThan(3000); // 3秒内加载
});
```

### 2. 无障碍测试
```javascript
import { test, expect } from '@playwright/test';
import { axeBuilder } from '@axe-core/playwright';

test('无障碍测试', async ({ page }) => {
  await page.goto('/');
  const accessibilityScanResults = await new axeBuilder({ page }).analyze();
  expect(accessibilityScanResults.violations).toEqual([]);
});
```

### 3. 移动端测试
```javascript
test.use({
  ...devices['iPhone 12'],
});

test('移动端响应式', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.menu')).toHaveCSS('display', 'none');
});
```

## 总结

浏览器自动化测试是确保Web应用质量的关键环节。通过实施全面的测试套件，可以：

1. **提高代码质量** - 及早发现回归问题
2. **增强信心** - 确保核心功能正常工作
3. **加速开发** - 自动化重复测试任务
4. **改善协作** - 为团队提供明确的质量标准

本项目已提供基础测试框架和示例测试用例，可根据具体需求扩展测试覆盖。