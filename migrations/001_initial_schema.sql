-- 初始数据库模式，基于需求.md中的9个表结构
-- 所有表名和字段名使用英文，但保持与中文需求的对应关系

-- 1. 品种档案 (Plant Archive)
-- 记录所有种植品种的静态信息，主键：序号
CREATE TABLE plant_archive (
    id INTEGER PRIMARY KEY AUTOINCREMENT,  -- 序号
    short_name TEXT NOT NULL UNIQUE,       -- 简称，用户定义的品种代号
    full_name TEXT,                        -- 品种名称
    category TEXT,                         -- 种类（辣椒、番茄、南瓜等）
    variety_type TEXT,                     -- 品种类型（早熟、无限生长等）
    height_habit TEXT,                     -- 株高/习性
    fruit_features TEXT,                   -- 果实特征
    taste_usage TEXT,                      -- 口感/用途
    estimated_yield TEXT,                  -- 单株预估产量（如"0.5-0.8公斤"）
    notes TEXT,                            -- 备注
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 索引：简称必须唯一且用于关联
CREATE INDEX idx_plant_archive_short_name ON plant_archive(short_name);

-- 2. 生长日志第一批 (Growth Log Batch 1)
-- 记录第一批育苗（辣椒、番茄等）的动态事件，主键：序号（全局唯一）
CREATE TABLE growth_log_batch1 (
    id INTEGER PRIMARY KEY AUTOINCREMENT,  -- 序号
    plant_short_name TEXT NOT NULL,        -- 品种，必须与品种档案的简称匹配
    event_date DATE NOT NULL,              -- 日期，YYYY.MM.DD格式
    event_type TEXT NOT NULL CHECK(        -- 事件类型
        event_type IN ('播种', '出芽', '假植', '移栽', '死亡', '观察', '操作', '处理')
    ),
    quantity_location TEXT,                -- 数量/部位（如"8粒"、"6号位"、"1、2、4号杯"）
    details TEXT,                          -- 详情记录
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    -- 外键约束：确保植物简称存在于品种档案中
    FOREIGN KEY (plant_short_name) REFERENCES plant_archive(short_name)
);

-- 索引：支持全局排序规则（品种 > 日期 > 序号）
CREATE INDEX idx_growth_log_batch1_sorting ON growth_log_batch1(plant_short_name, event_date, id);

-- 3. 生长日志第二批 (Growth Log Batch 2)
-- 记录第二批育苗（瓜类、洛神花、秋葵等）的动态事件，结构与第一批相同但独立
CREATE TABLE growth_log_batch2 (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plant_short_name TEXT NOT NULL,
    event_date DATE NOT NULL,
    event_type TEXT NOT NULL CHECK(
        event_type IN ('播种', '出芽', '假植', '移栽', '死亡', '观察', '操作', '处理')
    ),
    quantity_location TEXT,
    details TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (plant_short_name) REFERENCES plant_archive(short_name)
);

CREATE INDEX idx_growth_log_batch2_sorting ON growth_log_batch2(plant_short_name, event_date, id);

-- 4. 产量记录 (Yield Records)
-- 记录采收量，结构待定（需求中为空）
CREATE TABLE yield_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plant_short_name TEXT NOT NULL,
    harvest_date DATE NOT NULL,
    quantity REAL,                         -- 产量（重量或数量）
    unit TEXT,                            -- 单位（公斤、个等）
    notes TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (plant_short_name) REFERENCES plant_archive(short_name)
);

-- 5. 出芽率与活苗率统计 (Germination and Survival Statistics)
-- 从生长日志汇总的出芽和存活情况，主键：批次+品种
-- 此表为计算表，可从生长日志动态生成，但存储以提升性能
CREATE TABLE germination_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    batch TEXT NOT NULL CHECK(batch IN ('第一批', '第二批')),  -- 批次
    plant_short_name TEXT NOT NULL,                           -- 品种简称
    seeds_sown INTEGER NOT NULL DEFAULT 0,                    -- 播种数
    seeds_germinated INTEGER NOT NULL DEFAULT 0,              -- 已出芽
    seeds_pending INTEGER NOT NULL DEFAULT 0,                 -- 待出芽（播种数 - 已出芽 - 已死亡种子数）
    seeds_dead INTEGER NOT NULL DEFAULT 0,                    -- 已死亡种子数
    germination_rate REAL,                                    -- 出芽率（计算字段）
    survival_rate REAL,                                       -- 定植前活苗率（计算字段）
    notes TEXT,                                               -- 备注（如"未出3、4号位；1号已死"）
    calculated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (plant_short_name) REFERENCES plant_archive(short_name),
    UNIQUE(batch, plant_short_name)
);

-- 6. 育苗以外植物记录 (Non-Seedling Plant Records)
-- 记录蓝莓、葡萄、韭菜、堆肥等非育苗植物的操作与观察
CREATE TABLE non_seedling_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plant_name TEXT NOT NULL,                -- 植物名称（如蓝莓、韭菜、堆肥）
    record_date DATE NOT NULL,               -- 日期，YYYY.MM.DD格式
    record_type TEXT NOT NULL CHECK(         -- 类型：操作、观察
        record_type IN ('操作', '观察')
    ),
    details TEXT NOT NULL,                   -- 详情
    notes TEXT,                              -- 备注
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 索引：按日期排序
CREATE INDEX idx_non_seedling_records_date ON non_seedling_records(record_date);

-- 7. 肥料与基质信息表 (Fertilizer and Substrate Information)
-- 记录肥料、基质、菌剂等材料信息，主键：名称
CREATE TABLE fertilizer_materials (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,               -- 名称
    category TEXT,                           -- 类别（肥料、基质、菌剂等）
    description TEXT,                        -- 描述
    usage_instructions TEXT,                 -- 使用说明
    notes TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 8. 种植容器尺寸清单 (Container Size List)
-- 记录容器类型、尺寸、数量，主键：序号
CREATE TABLE container_sizes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    container_type TEXT NOT NULL,            -- 容器类型
    dimensions TEXT,                         -- 尺寸
    quantity INTEGER,                        -- 数量
    notes TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 9. 当前待办与重要提醒 (Todo and Important Reminders)
-- 动态备忘，非结构化
CREATE TABLE todo_reminders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content TEXT NOT NULL,                   -- 内容
    priority TEXT CHECK(priority IN ('高', '中', '低')),  -- 优先级
    due_date DATE,                           -- 截止日期
    completed BOOLEAN DEFAULT FALSE,         -- 是否完成
    completed_at TIMESTAMP,
    notes TEXT,                               -- 备注
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 触发器：更新时自动更新updated_at字段
CREATE TRIGGER update_plant_archive_timestamp AFTER UPDATE ON plant_archive
BEGIN
    UPDATE plant_archive SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- 视图：用于计算出芽率和活苗率（如果需要）
CREATE VIEW v_germination_stats_calculated AS
SELECT
    batch,
    plant_short_name,
    seeds_sown,
    seeds_germinated,
    seeds_pending,
    seeds_dead,
    -- 出芽率 = (已出芽 / 播种数) * 100%
    CASE
        WHEN seeds_sown > 0 THEN ROUND((seeds_germinated * 100.0 / seeds_sown), 2)
        ELSE 0
    END as germination_rate,
    -- 定植前活苗率 = (已出芽且未死亡苗数 / 播种数) * 100%
    CASE
        WHEN seeds_sown > 0 THEN ROUND(((seeds_germinated - seeds_dead) * 100.0 / seeds_sown), 2)
        ELSE 0
    END as survival_rate,
    notes,
    calculated_at
FROM germination_stats;

-- 测试数据已移至002_import_initial_data.sql