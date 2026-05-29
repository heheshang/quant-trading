# Quant Trading UI Redesign Specification
# 基于 UI/UX Pro Max — 最流行金融仪表盘设计语言

## 1. 设计语言体系

### 1.1 美学方向
**风格参考**：Bloomberg Terminal + Linear App 混合
- 高信息密度 Data-Dense 布局
- 暗色 OLED 友好主题
- 微妙的层次感和精致阴影
- 专业金融终端的视觉语言

### 1.2 色彩系统 (Financial Dashboard Palette)

```scss
// === 核心 ===
--color-bg: #0A0E17;              // 深空背景
--color-surface: #111827;         // 卡片/面板
--color-surface-elevated: #1A2035; // 悬浮元素
--color-deep-bg: #050810;         // 最深背景

// === 文字 ===
--color-text-primary: #F1F5F9;     // 主文字
--color-text-secondary: #94A3B8;  // 次要文字
--color-text-tertiary: #64748B;   // 辅助文字

// === 主色 (Trust Blue) ===
--color-primary: #3B82F6;
--color-primary-hover: #60A5FA;
--color-primary-light: rgba(59, 130, 246, 0.15);

// === 功能色 ===
--color-buy: #10B981;             // 买入/盈利
--color-sell: #EF4444;            // 卖出/亏损
--color-buy-bg: rgba(16, 185, 129, 0.12);
--color-sell-bg: rgba(239, 68, 68, 0.12);

// === 边框/分隔 ===
--color-border: rgba(255, 255, 255, 0.08);
--color-border-hover: rgba(255, 255, 255, 0.15);

// === 状态 ===
--color-warning: #F59E0B;
--color-info: #6366F1;
--color-success: #10B981;
--color-error: #EF4444;

// === 图表 ===
--color-chart-grid: rgba(255, 255, 255, 0.05);
--color-chart-line: #3B82F6;
--color-chart-fill: rgba(59, 130, 246, 0.1);
```

### 1.3 字体系统 (Geometric Modern)

```scss
// Google Fonts
@import url('https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&family=Work+Sans:wght@300;400;500;600&display=swap');

// 字体变量
--font-heading: 'Outfit', sans-serif;
--font-body: 'Work Sans', sans-serif;
--font-mono: 'JetBrains Mono', 'SF Mono', monospace;

// 字重规范
// 300 - Light: 大数字/数据展示
// 400 - Regular: 正文
// 500 - Medium: 标签/小标题
// 600 - SemiBold: 导航/按钮
// 700 - Bold: 页面标题/强调
```

### 1.4 间距系统

```scss
// 8px 基准网格
--space-1: 4px;
--space-2: 8px;
--space-3: 12px;
--space-4: 16px;
--space-5: 20px;
--space-6: 24px;
--space-8: 32px;
--space-10: 40px;
--space-12: 48px;

// 圆角
--radius-sm: 6px;
--radius-md: 8px;
--radius-lg: 12px;
--radius-xl: 16px;
```

### 1.5 阴影系统

```scss
--shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.3);
--shadow-md: 0 4px 12px rgba(0, 0, 0, 0.4);
--shadow-lg: 0 8px 24px rgba(0, 0, 0, 0.5);
--shadow-card: 0 2px 8px rgba(0, 0, 0, 0.3);
--shadow-card-hover: 0 8px 24px rgba(0, 0, 0, 0.4);
--shadow-glow-primary: 0 0 20px rgba(59, 130, 246, 0.25);
--shadow-glow-buy: 0 0 12px rgba(16, 185, 129, 0.3);
--shadow-glow-sell: 0 0 12px rgba(239, 68, 68, 0.3);
```

### 1.6 过渡动画

```scss
--transition-fast: 120ms ease;
--transition-base: 200ms ease;
--transition-slow: 300ms ease;
--transition-spring: 300ms cubic-bezier(0.34, 1.56, 0.64, 1);
```

## 2. 布局规范

### 2.1 页面结构
```
┌─────────────────────────────────────────────────────────────┐
│ Header (48px, sticky)                                       │
├──────────┬──────────────────────────────────────────────────┤
│ Sidebar  │ Content Area                                     │
│ (240px)  │ max-width: 1344px, padding: 24px                │
│          │                                                  │
│          │ ┌─────────────────────────────────────────────┐ │
│          │ │ Page Header                                 │ │
│          │ └─────────────────────────────────────────────┘ │
│          │                                                  │
│          │ ┌─────────────────────────────────────────────┐ │
│          │ │ Main Content                                │ │
│          │ │                                             │ │
│          │ └─────────────────────────────────────────────┘ │
└──────────┴──────────────────────────────────────────────────┘
```

### 2.2 响应式断点
```
Mobile:  < 640px   (单列布局，侧边栏 drawer)
Tablet:  640-1024px (两列)
Desktop: > 1024px  (完整布局)
```

## 3. 组件规范

### 3.1 统计卡片 (Stat Card)
```scss
// 尺寸: 自适应宽度, 高度 ~120px
// 内边距: 20px
// 背景: var(--color-surface)
// 边框: 1px solid var(--color-border)
// 圆角: var(--radius-lg)

// 状态
default:   边框正常，无阴影
hover:     边框变亮，translateY(-2px)，阴影加深
active:    左边框 3px 主色高亮

// 内容结构
┌────────────────────────────┐
│ [图标] 标签文字             │  <- 12px, tertiary color
│                            │
│ 大数字                     │  <- 28px, bold, primary color
│                            │
│ 变化量 / 描述文字          │  <- 12px, semantic color
└────────────────────────────┘
```

### 3.2 数据表格
```scss
// 表头
background: transparent
文字: tertiary color, 11px, uppercase, letter-spacing: 0.5px
边框: 只有底边框 1px var(--color-border)

// 数据行
hover: background 变亮为 rgba(255,255,255,0.02)
边框: 只有底边框

// 涨跌色
买入/盈利: var(--color-buy)
卖出/亏损: var(--color-sell)
```

### 3.3 按钮
```scss
// Primary
background: var(--color-primary)
hover: var(--color-primary-hover) + 轻微发光
active: scale(0.98)
disabled: opacity 0.5

// Secondary/Ghost
background: transparent
border: 1px solid var(--color-border)
hover: background rgba(255,255,255,0.05)

// 尺寸
sm:  28px height, 12px padding
md:  36px height, 16px padding (default)
lg:  44px height, 20px padding
```

### 3.4 导航栏 (Sidebar)
```scss
// 宽度: 240px (折叠时 64px)
// 背景: #0A0E17 + 顶部 1px 边框渐变
// Logo 区域: 48px height

.nav-item {
  height: 40px
  padding: 0 12px
  border-radius: var(--radius-md)
  gap: 12px
  
  &:hover {
    background: rgba(255,255,255,0.05)
  }
  
  &.active {
    background: var(--color-primary-light)
    color: var(--color-primary)
    font-weight: 500
    
    &::before {
      // 左侧 3px 指示条
      content: ''
      position: absolute
      left: 0
      width: 3px
      height: 20px
      background: var(--color-primary)
      border-radius: 0 2px 2px 0
    }
  }
}
```

### 3.5 图表卡片
```scss
.chart-card {
  background: var(--color-surface)
  border: 1px solid var(--color-border)
  border-radius: var(--radius-lg)
  padding: 20px
  
  .chart-header {
    display: flex
    justify-content: space-between
    align-items: center
    margin-bottom: 16px
    
    h3 {
      font-size: 15px
      font-weight: 600
    }
  }
}
```

## 4. 交互规范

### 4.1 悬停状态
- 卡片: `transform: translateY(-2px)` + 阴影加深
- 按钮: 颜色变亮 + 轻微发光
- 表格行: 背景微亮
- 导航项: 背景色 + 文字变亮

### 4.2 点击反馈
- 按钮: `scale(0.98)` 100ms
- 卡片: 无 transform（避免与 hover 冲突）

### 4.3 加载状态
- 骨架屏 shimmer 动画
- 颜色: 深色渐变到浅色再渐变回深色

### 4.4 空状态
- 居中图标 + 说明文字
- 辅助操作入口

## 5. 交付检查清单

### 5.1 视觉检查
- [ ] 颜色对比度 WCAG AA (文字 vs 背景)
- [ ] 涨跌色一致性 (绿买红卖)
- [ ] 卡片阴影层次感
- [ ] 字体层级清晰

### 5.2 交互检查
- [ ] hover/focus/active 状态完整
- [ ] 按钮点击反馈
- [ ] 表格行 hover 效果
- [ ] 加载/空状态处理

### 5.3 响应式检查
- [ ] Mobile (< 640px) 侧边栏 drawer
- [ ] Tablet (640-1024px) 合适间距
- [ ] Desktop (> 1024px) 完整布局