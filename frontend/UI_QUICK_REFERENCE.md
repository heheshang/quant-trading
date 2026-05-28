# Frontend UI Quick Reference Guide

## 🎨 Design Tokens

### Colors
```scss
// Primary
var(--color-accent)         // #7170ff - Main brand color
var(--color-accent-hover)   // #8b8aff - Hover state
var(--color-accent-light)   // rgba(113, 112, 255, 0.15) - Subtle backgrounds

// Semantic
var(--color-buy)            // #10b981 - Success/Buy
var(--color-sell)           // #e5484d - Error/Sell
var(--color-warning)        // #f5a623 - Warning
var(--color-info)           // #60a5fa - Info

// Text
var(--color-text-primary)   // #f7f8f8 - Main text
var(--color-text-secondary) // #d0d6e0 - Secondary
var(--color-text-tertiary)  // #8a8f98 - Tertiary/Muted
```

### Shadows
```scss
var(--shadow-sm)    // 0 1px 2px rgba(0, 0, 0, 0.3)
var(--shadow-md)    // 0 4px 12px rgba(0, 0, 0, 0.4)
var(--shadow-lg)    // 0 8px 24px rgba(0, 0, 0, 0.5)
var(--shadow-glow)  // 0 0 20px rgba(113, 112, 255, 0.3)
var(--shadow-card)      // 0 2px 8px rgba(0, 0, 0, 0.4)
var(--shadow-card-hover) // 0 4px 16px rgba(0, 0, 0, 0.5)
```

### Gradients
```scss
var(--gradient-accent)   // Purple gradient (primary actions)
var(--gradient-success)  // Green gradient (buy/success)
var(--gradient-danger)   // Red gradient (sell/danger)
var(--gradient-surface)  // Subtle surface gradient
```

### Transitions
```scss
var(--transition-fast)   // 150ms ease
var(--transition-base)   // 200ms ease
var(--transition-slow)   // 300ms ease
```

---

## 🛠️ Utility Classes

### Text Utilities
```html
<span class="text-mono">Monospace text</span>
<span class="text-gradient">Gradient text</span>
<span class="text-primary">Primary color</span>
<span class="text-secondary">Secondary color</span>
<span class="text-buy">Buy/Green color</span>
<span class="text-sell">Sell/Red color</span>
```

### Card Utilities
```html
<div class="card-elevated">
  <!-- Card with hover lift effect -->
</div>
```

### Animation Utilities
```html
<div class="animate-fade-in">Fade in on mount</div>
<div class="animate-slide-up">Slide up on mount</div>
<div class="animate-pulse-subtle">Subtle pulse animation</div>
```

### Effects
```html
<div class="glass-effect">Glassmorphism blur</div>
<button class="btn-glow">Button with ripple</button>
<div class="focus-ring">Enhanced focus ring</div>
```

---

## 📝 Common Patterns

### Enhanced Card
```vue
<template>
  <div class="enhanced-card">
    <div class="card-header">
      <h3>Title</h3>
    </div>
    <div class="card-body">
      Content here
    </div>
  </div>
</template>

<style scoped lang="scss">
.enhanced-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 12px;
  padding: 20px;
  box-shadow: var(--shadow-card);
  transition: all var(--transition-base);

  &:hover {
    box-shadow: var(--shadow-card-hover);
    border-color: var(--color-border-hover);
    transform: translateY(-2px);
  }
}
</style>
```

### Gradient Button
```vue
<template>
  <button class="gradient-btn">
    Click Me
  </button>
</template>

<style scoped lang="scss">
.gradient-btn {
  background: var(--gradient-accent);
  color: white;
  border: none;
  border-radius: 10px;
  padding: 12px 24px;
  font-weight: 600;
  box-shadow: 0 4px 12px rgba(113, 112, 255, 0.3);
  transition: all var(--transition-fast);

  &:hover {
    transform: translateY(-2px);
    box-shadow: 0 6px 20px rgba(113, 112, 255, 0.4);
  }

  &:active {
    transform: translateY(0);
  }
}
</style>
```

### Glassmorphism Header
```vue
<template>
  <header class="glass-header">
    Header Content
  </header>
</template>

<style scoped lang="scss">
.glass-header {
  background: rgba(8, 9, 10, 0.95);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  border-bottom: 1px solid var(--color-border);
  box-shadow: var(--shadow-sm);
}
</style>
```

### Animated Skeleton Loader
```vue
<template>
  <div class="skeleton-card">
    <div class="skeleton-line"></div>
    <div class="skeleton-line short"></div>
  </div>
</template>

<style scoped lang="scss">
.skeleton-line {
  height: 16px;
  border-radius: 4px;
  background: linear-gradient(
    90deg,
    var(--color-surface-elevated) 25%,
    rgba(255,255,255,0.05) 50%,
    var(--color-surface-elevated) 75%
  );
  background-size: 200% 100%;
  animation: shimmer 1.5s ease-in-out infinite;

  &.short {
    width: 60%;
  }
}

@keyframes shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}
</style>
```

---

## 🎯 Component Styling Tips

### Tables
```scss
:deep(.el-table) {
  --el-table-row-hover-bg-color: rgba(113, 112, 255, 0.05);

  .el-table__row {
    transition: all var(--transition-fast);

    &:hover {
      transform: scale(1.005);
    }
  }
}
```

### Forms
```scss
:deep(.el-input__wrapper) {
  background: var(--color-surface-elevated) !important;
  border-radius: 8px;
  transition: all var(--transition-fast);

  &:hover {
    box-shadow: 0 0 0 1px var(--color-border-hover) inset !important;
  }

  &.is-focus {
    box-shadow:
      0 0 0 1px var(--color-accent) inset,
      0 0 16px rgba(113, 112, 255, 0.2) !important;
  }
}
```

### Tabs
```scss
:deep(.el-tabs__active-bar) {
  background: var(--gradient-accent);
  height: 3px;
  border-radius: 3px 3px 0 0;
}

:deep(.el-tabs__item) {
  transition: all var(--transition-fast);

  &.is-active {
    color: var(--color-accent);
    font-weight: 600;
  }
}
```

---

## 🌟 Key Animations

### Fade In
```scss
@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

.element {
  animation: fadeIn 0.2s ease-out;
}
```

### Slide Up
```scss
@keyframes slideUp {
  from {
    opacity: 0;
    transform: translateY(20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.element {
  animation: slideUp 0.2s ease-out;
}
```

### Pulse Subtle
```scss
@keyframes pulseSubtle {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.7; }
}

.element {
  animation: pulseSubtle 2s ease-in-out infinite;
}
```

### Glow Effect
```scss
@keyframes glow {
  0%, 100% {
    box-shadow: 0 0 5px rgba(113, 112, 255, 0.5);
  }
  50% {
    box-shadow: 0 0 20px rgba(113, 112, 255, 0.8);
  }
}

.element {
  animation: glow 3s ease-in-out infinite;
}
```

---

## 📱 Responsive Breakpoints

```scss
// Desktop first approach
.element {
  // Desktop styles (> 1200px)

  @media (max-width: 1200px) {
    // Large tablet
  }

  @media (max-width: 768px) {
    // Tablet
  }

  @media (max-width: 480px) {
    // Mobile
  }
}
```

### Common Grid Patterns
```scss
// 4 column grid
.grid-4 {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;

  @media (max-width: 768px) {
    grid-template-columns: repeat(2, 1fr);
  }

  @media (max-width: 480px) {
    grid-template-columns: 1fr;
  }
}

// 2 column grid
.grid-2 {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 16px;

  @media (max-width: 1024px) {
    grid-template-columns: 1fr;
  }
}
```

---

## ♿ Accessibility Best Practices

### Focus States
```scss
.focus-ring {
  &:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }
}
```

### Reduced Motion
```scss
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

### Color Contrast
- Always test with contrast checker
- Minimum 4.5:1 for normal text
- Minimum 3:1 for large text (18px+ or 14px bold)

---

## 🚀 Performance Tips

### GPU Acceleration
Use these properties for smooth animations:
- `transform` (translate, scale, rotate)
- `opacity`
- `filter` (blur, drop-shadow)

Avoid animating:
- `width`, `height` (use transform: scale)
- `top`, `left` (use transform: translate)
- `margin`, `padding`

### Efficient Selectors
```scss
// Good - specific
.component .element { }

// Bad - too generic
div > span { }
```

### Will-Change (Use Sparingly)
```scss
.animated-element {
  will-change: transform, opacity;
}
```

---

## 🧪 Testing Checklist

Before deploying UI changes:

- [ ] Test on Chrome, Firefox, Safari, Edge
- [ ] Verify mobile responsiveness
- [ ] Check keyboard navigation
- [ ] Test with screen reader
- [ ] Verify color contrast ratios
- [ ] Test animations at 60fps
- [ ] Check loading states
- [ ] Verify error states
- [ ] Test with slow network
- [ ] Validate TypeScript types

---

## 📚 Resources

### Internal Documentation
- `/frontend/UI_OPTIMIZATION_SUMMARY.md` - Complete overview
- `/frontend/UI_ENHANCEMENT_GUIDE.md` - Visual comparisons
- `/frontend/src/assets/styles/variables.scss` - Design tokens
- `/frontend/src/assets/styles/global.scss` - Global utilities

### External Resources
- [Element Plus Docs](https://element-plus.org/)
- [Vue.js Style Guide](https://vuejs.org/style-guide/)
- [WCAG Guidelines](https://www.w3.org/WAI/WCAG21/quickref/)
- [Can I Use](https://caniuse.com/) - Browser compatibility

---

**Last Updated**: 2026-05-28
**Version**: 1.0.0
