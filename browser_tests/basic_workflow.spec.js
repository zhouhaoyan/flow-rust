import { test, expect } from '@playwright/test';

test.describe('Flower Rust 应用浏览器自动化测试', () => {
  test('应该能访问登录页面', async ({ page }) => {
    await page.goto('/login.html');
    await expect(page).toHaveTitle('登录 - Flower Rust');

    // 检查页面元素
    await expect(page.locator('h1')).toHaveText('登录');
    await expect(page.locator('input[name="username"]')).toBeVisible();
    await expect(page.locator('input[name="password"]')).toBeVisible();
    await expect(page.locator('button[type="submit"]')).toBeVisible();
  });

  test('应该能成功登录', async ({ page }) => {
    await page.goto('/login.html');

    // 填写登录表单
    await page.fill('input[name="username"]', 'admin');
    await page.fill('input[name="password"]', 'admin');

    // 提交表单
    await page.click('button[type="submit"]');

    // 等待跳转并验证
    await page.waitForURL('**/index.html');
    await expect(page).toHaveTitle('Flower Rust - 植物记录系统');

    // 检查是否显示欢迎信息
    await expect(page.locator('h1')).toContainText('植物记录系统');
  });

  test('应该能提交自然语言记录', async ({ page }) => {
    // 先登录
    await page.goto('/login.html');
    await page.fill('input[name="username"]', 'admin');
    await page.fill('input[name="password"]', 'admin');
    await page.click('button[type="submit"]');
    await page.waitForURL('**/index.html');

    // 找到自然语言输入框并提交
    await page.fill('#naturalLanguageInput', '辣椒出芽了');
    await page.click('#submitBtn');

    // 等待API响应
    const response = await page.waitForResponse('**/api/record');
    expect(response.ok()).toBeTruthy();

    // 检查是否显示确认提示
    await expect(page.locator('#recordList')).toContainText('等待确认');
  });

  test('应该能查看记录列表', async ({ page }) => {
    // 登录
    await page.goto('/login.html');
    await page.fill('input[name="username"]', 'admin');
    await page.fill('input[name="password"]', 'admin');
    await page.click('button[type="submit"]');
    await page.waitForURL('**/index.html');

    // 导航到记录列表
    await page.goto('/index.html#records');

    // 检查记录列表容器
    await expect(page.locator('#recordList')).toBeVisible();

    // 尝试刷新记录
    await page.click('#refreshBtn');

    // 检查API调用
    const response = await page.waitForResponse('**/api/records');
    expect(response.ok()).toBeTruthy();
  });

  test('应该能访问管理面板', async ({ page }) => {
    // 登录
    await page.goto('/login.html');
    await page.fill('input[name="username"]', 'admin');
    await page.fill('input[name="password"]', 'admin');
    await page.click('button[type="submit"]');
    await page.waitForURL('**/index.html');

    // 导航到管理面板
    await page.goto('/admin.html');
    await expect(page).toHaveTitle('管理面板 - Flower Rust');

    // 检查所有数据表标签页
    await expect(page.locator('nav.tabs')).toBeVisible();

    // 检查至少有一个标签页
    const tabs = page.locator('nav.tabs button');
    await expect(tabs).toHaveCount(9); // 9个数据表
  });

  test('应该能查看品种档案数据', async ({ page }) => {
    // 登录并进入管理面板
    await page.goto('/login.html');
    await page.fill('input[name="username"]', 'admin');
    await page.fill('input[name="password"]', 'admin');
    await page.click('button[type="submit"]');
    await page.waitForURL('**/index.html');
    await page.goto('/admin.html');

    // 点击品种档案标签页
    await page.click('button:has-text("品种档案")');

    // 等待数据加载
    const response = await page.waitForResponse('**/api/plant-archive');
    expect(response.ok()).toBeTruthy();

    // 检查数据表格
    await expect(page.locator('#plantArchiveTable')).toBeVisible();
  });
});