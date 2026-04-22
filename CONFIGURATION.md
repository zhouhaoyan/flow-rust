# DeepSeek API 配置指南

## 1. 获取 DeepSeek API 密钥

1. 访问 [DeepSeek 官方平台](https://platform.deepseek.com/)
2. 注册账号并登录
3. 在控制台中创建新的 API 密钥
4. 复制您的 API 密钥（形如 `sk-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx`）

## 2. 配置环境变量

### 方法一：直接编辑 `.env` 文件

编辑项目根目录下的 `.env` 文件：

```env
# 将 'mock' 替换为您的真实 API 密钥
DEEPSEEK_API_KEY=your_real_api_key_here

# DeepSeek API 基础 URL（通常不需要修改）
DEEPSEEK_BASE_URL=https://api.deepseek.com

# 服务器端口
PORT=3000
```

### 方法二：使用环境变量（推荐用于生产环境）

```bash
export DEEPSEEK_API_KEY=your_real_api_key_here
export DEEPSEEK_BASE_URL=https://api.deepseek.com
export PORT=3000
```

## 3. 验证配置

启动服务器检查配置是否正确：

```bash
cargo run
```

如果配置成功，启动日志中**不会**显示 "Using mock DeepSeek client"。

## 4. 测试 API 连接

使用以下命令测试 API 是否正常工作：

```bash
curl -X POST http://localhost:3000/api/record \
  -H "Content-Type: application/json" \
  -d '{"text": "黄瓜有一粒露白"}'
```

正常响应应包含 `"success": true`。

## 5. 安全注意事项

1. **不要将 API 密钥提交到版本控制系统**
   - 确保 `.env` 文件在 `.gitignore` 中
   - 使用 `.env.example` 作为模板

2. **密钥权限管理**
   - 为不同环境使用不同的 API 密钥
   - 定期轮换 API 密钥

3. **使用环境变量（推荐）**
   ```bash
   # 在部署时设置环境变量
   export DEEPSEEK_API_KEY=your_key_here
   ```

## 6. 故障排除

### 常见问题

1. **API 调用失败**
   - 检查 API 密钥是否正确
   - 验证网络连接
   - 确认账户余额充足

2. **解析错误**
   - 确保输入文本是中文植物相关描述
   - 检查 DeepSeek 服务状态

3. **服务器启动失败**
   - 检查端口 3000 是否被占用
   - 验证环境变量格式

## 7. 切换到模拟模式（开发测试）

如需切换回模拟模式，将 `.env` 文件中的 `DEEPSEEK_API_KEY` 设为 `"mock"`：

```env
DEEPSEEK_API_KEY=mock
```